use std::{ops::ControlFlow, time::Duration};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use td_util_buck::types::ProjectRelativePath;
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
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::{BuildRequest, buck_build},
    repo::sapling::status::Status,
};

/// Task phase when in buck2 build
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum TaskPhase {
    DownloadingSource,
    RunningBuild,
}

/// Message protocol for WebSocket communication between worker and server.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum WSMessage {
    // Worker -> Server messages
    Register {
        id: String,
        hostname: String,
        orion_version: String,
    },
    Heartbeat,
    // Sent when a task is in the build process and its execution phase changes.
    TaskPhaseUpdate {
        id: String,
        phase: TaskPhase,
    },
    TaskAck {
        id: String,
        success: bool,
        message: String,
    },
    BuildOutput {
        id: String,
        output: String,
    },
    BuildComplete {
        id: String,
        success: bool,
        exit_code: Option<i32>,
        message: String,
    },
    // Server -> Worker messages
    Task {
        id: String,
        repo: String,
        cl_link: String,
        changes: Vec<Status<ProjectRelativePath>>,
    },
}

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
/// - Heartbeat transmission to maintain connection
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
    let orion_version = env!("CARGO_PKG_VERSION").to_string(); // Get from Cargo.toml

    let internal_tx_clone = internal_tx.clone();
    let heartbeat_task = tokio::spawn(async move {
        tracing::info!("Registering with worker ID: {}", worker_id_clone);
        if internal_tx_clone
            .send(WSMessage::Register {
                id: worker_id_clone,
                hostname: hostname_clone,
                orion_version,
            })
            .is_err()
        {
            tracing::error!("Failed to queue register message. Internal channel closed.");
            return;
        }
        let heartbeat_interval = Duration::from_secs(30);
        loop {
            tokio::time::sleep(heartbeat_interval).await;
            tracing::debug!("Sending heartbeat...");
            if internal_tx_clone.send(WSMessage::Heartbeat).is_err() {
                tracing::warn!("Failed to queue heartbeat message. Internal channel closed.");
                break;
            }
        }
    });

    let mut ws_sender = ws_sender;
    let send_task = tokio::spawn(async move {
        while let Some(msg) = internal_rx.recv().await {
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
                Err(e) => {
                    tracing::error!("Failed to serialize WSMessage: {}", e);
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
        _ = heartbeat_task => tracing::info!("Heartbeat task finished."),
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
                        WSMessage::Task {
                            id,
                            repo,
                            cl_link: cl,
                            changes,
                        } => {
                            tracing::info!("Received task: id={}", id);
                            tokio::spawn(async move {
                                let task_id_uuid = match Uuid::parse_str(&id) {
                                    Ok(uuid) => uuid,
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to parse task id '{}' as Uuid: {}. Aborting task.",
                                            id,
                                            e
                                        );
                                        return;
                                    }
                                };

                                let build_result = buck_build(
                                    task_id_uuid,
                                    BuildRequest { repo, cl, changes },
                                    sender.clone(),
                                )
                                .await;

                                if let Err(e) = sender.send(WSMessage::TaskAck {
                                    id,
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
