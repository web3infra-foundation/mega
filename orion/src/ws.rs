use std::{ops::ControlFlow, time::Duration};

use api_model::buck2::ws::WSMessage;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::{
        mpsc,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::protocol::Message,
};
use uuid::Uuid;

use crate::api::buck_build;

/// Manages persistent WebSocket connection with automatic reconnection.
///
/// Handles connection establishment, registration, heartbeat, and task processing.
/// Implements exponential backoff for reconnection attempts.
///
/// # Arguments
/// * `server_addr` - WebSocket server endpoint URL
/// * `worker_id` - Unique identifier for this worker instance
pub async fn run_client(server_addr: String, worker_id: String) {
    let mut reconnect_delay = Duration::from_secs(1);
    const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);

    loop {
        tracing::info!("Attempting to connect to server: {}", server_addr);
        match connect_async(&server_addr).await {
            Ok((ws_stream, response)) => {
                tracing::info!(
                    "WebSocket handshake successful. Server response: {:?}",
                    response.status()
                );
                // Reset reconnect delay after successful connection
                reconnect_delay = Duration::from_secs(1);
                // Handle the active connection
                handle_connection(ws_stream, worker_id.clone(), server_addr.clone()).await;
                tracing::warn!("Disconnected from server.");
            }
            Err(e) => {
                tracing::error!(
                    "WebSocket handshake failed: {}. Retrying in {:?}...",
                    e,
                    reconnect_delay
                );
            }
        }
        // Wait before attempting to reconnect
        tokio::time::sleep(reconnect_delay).await;
        reconnect_delay = (reconnect_delay * 2).min(MAX_RECONNECT_DELAY);
    }
}

/// Processes an established WebSocket connection.
///
/// Coordinates three concurrent tasks:
/// - Heartbeat on a timer with priority over build output
/// - Message sending from internal channels
/// - Message receiving and processing from server
///
/// # Arguments
/// * `ws_stream` - Established WebSocket connection
/// * `worker_id` - Worker identifier for registration
async fn handle_connection(
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    worker_id: String,
    server_addr: String,
) {
    let (ws_sender, mut ws_receiver) = ws_stream.split();
    let (internal_tx, mut internal_rx): (UnboundedSender<WSMessage>, UnboundedReceiver<WSMessage>) =
        mpsc::unbounded_channel();

    let worker_id_clone = worker_id.clone();
    let hostname_clone = server_addr.clone();
    let orion_version = env!("CARGO_PKG_VERSION").to_string();

    let send_task = tokio::spawn(async move {
        // Heartbeats must not sit behind unbounded build output/status batches: the server drops
        // workers after ~90s without a Heartbeat (see orion-server health check).
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));

        tracing::info!("Registering with worker ID: {}", worker_id_clone);
        let register = WSMessage::Register {
            id: worker_id_clone,
            hostname: hostname_clone,
            orion_version,
        };
        let register_str = match serde_json::to_string(&register) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to serialize Register: {}", e);
                return;
            }
        };
        let mut ws_sender = ws_sender;
        if let Err(e) = ws_sender.send(Message::Text(register_str.into())).await {
            tracing::error!(
                "Failed to send Register to server: {}. Terminating send task.",
                e
            );
            return;
        }

        loop {
            tokio::select! {
                biased;
                _ = heartbeat_interval.tick() => {
                    tracing::info!("Sending heartbeat...");
                    match serde_json::to_string(&WSMessage::Heartbeat) {
                        Ok(payload) => {
                            if let Err(e) = ws_sender.send(Message::Text(payload.into())).await {
                                tracing::warn!("Failed to send heartbeat on WebSocket: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to serialize Heartbeat: {}", e);
                        }
                    }
                }
                maybe_msg = internal_rx.recv() => {
                    let Some(msg) = maybe_msg else { break; };
                    match serde_json::to_string(&msg) {
                        Ok(msg_str) => {
                            if let Err(e) = ws_sender.send(Message::Text(msg_str.into())).await {
                                tracing::error!(
                                    "Failed to send message to server: {}. Terminating send task.",
                                    e
                                );
                                break;
                            }
                        }
                        Err(e) => tracing::error!("Failed to serialize WSMessage: {}", e),
                    }
                }
            }
        }
    });

    let internal_tx_clone = internal_tx.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if process_server_message(msg, internal_tx_clone.clone())
                .await
                .is_break()
            {
                break;
            }
        }
    });

    // Wait for any task to complete
    tokio::select! {
        _ = send_task => tracing::info!("Send task finished."),
        _ = recv_task => tracing::info!("Receive task finished."),
    }
}

/// Processes incoming server messages and handles task execution.
///
/// Handles different message types including Task assignments and connection management.
/// For Task messages, spawns build processes and sends acknowledgments.
///
/// # Arguments
/// * `msg` - WebSocket message received from server
/// * `tx` - Channel for sending response messages
///
/// # Returns
/// * `ControlFlow::Continue(())` - Continue message processing
/// * `ControlFlow::Break(())` - Terminate connection
async fn process_server_message(
    msg: Message,
    sender: UnboundedSender<WSMessage>,
) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            match serde_json::from_str::<WSMessage>(&t) {
                Ok(ws_msg) => {
                    tracing::info!("Received message from server: {:?}", ws_msg);
                    match ws_msg {
                        WSMessage::TaskBuild {
                            build_id,
                            repo,
                            cl_link,
                            changes,
                        } => {
                            tracing::info!("Received task: id={}", build_id);
                            tokio::spawn(async move {
                                let task_id_uuid = match Uuid::parse_str(&build_id) {
                                    Ok(uuid) => uuid,
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to parse task id '{}' as Uuid: {}. Aborting task.",
                                            build_id,
                                            e
                                        );
                                        return;
                                    }
                                };

                                let build_result = buck_build(
                                    task_id_uuid,
                                    cl_link,
                                    repo,
                                    changes,
                                    sender.clone(),
                                )
                                .await;

                                if let Err(e) = sender.send(WSMessage::TaskAck {
                                    build_id,
                                    success: build_result.success,
                                    message: build_result.message.clone(),
                                }) {
                                    tracing::error!("Failed to send TaskAck: {}", e);
                                }
                            });
                        }
                        // Log unexpected message types
                        _ => {
                            tracing::warn!("Received unexpected message from server: {:?}", ws_msg);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error deserializing message from server: {}", e);
                }
            }
        }
        Message::Close(c) => {
            tracing::warn!("Server sent close frame: {:?}", c);
            return ControlFlow::Break(());
        }
        _ => {} // Ignore Binary, Ping, Pong and other message types
    }
    ControlFlow::Continue(())
}
