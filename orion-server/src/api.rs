use axum::body::Bytes;
use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::{any, post};
use axum::{Json, Router};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use rand::seq::SliceRandom;
use scopeguard::defer;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct BuildRequest {
    repo: String,
    target: String,
    args: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub enum WSMessage {
    Task {
        id: String,
        repo: String,
        target: String,
        args: Option<Vec<String>>,
    },
    TaskAck {
        id: String,
        success: bool,
        message: String,
    },
    BuildOutput,
    BuildComplete,
}

#[derive(Clone)]
pub struct AppState {
    pub clients: Arc<DashMap<String, UnboundedSender<WSMessage>>>,
}

pub fn routers() -> Router<AppState> {
    Router::new()
        .route("/ws", any(ws_handler))
        .route("/task", post(task_handler))
}

async fn task_handler(
    State(state): State<AppState>,
    Json(req): Json<BuildRequest>,
) -> impl IntoResponse {
    let client_ids: Vec<_> = state
        .clients
        .iter()
        .map(|entry| entry.key().clone())
        .collect();

    if client_ids.is_empty() {
        return axum_extra::json!({"error": "No clients connected"});
    }

    let mut rng = rand::thread_rng();
    let chosen_id = client_ids.choose(&mut rng).unwrap();

    let msg = WSMessage::Task {
        id: Uuid::now_v7().to_string(),
        repo: req.repo,
        target: req.target,
        args: req.args,
    };

    state.clients.get(chosen_id).unwrap().send(msg).unwrap(); // TODO client maybe disconnected

    axum_extra::json!({"client_id": chosen_id})
}
async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    println!("{addr} connected.");
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(mut socket: WebSocket, who: SocketAddr, state: AppState) {
    let client_id = Uuid::now_v7().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<WSMessage>();
    state.clients.insert(client_id.clone(), tx);
    defer! {
        println!("clean Client {who}.");
        state.clients.remove(&client_id);
    }

    // send a ping (unsupported by some browsers) just to kick things off and get a response
    if socket
        .send(Message::Ping(Bytes::from_static(b"Server hello")))
        .await
        .is_ok()
    {
        println!("Pinged {who}...");
    } else {
        println!("Could not send ping {who}!");
        return; // exit
    }

    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut sender, mut receiver) = socket.split();

    // Spawn a task that will push several messages to the client (does not matter what client does)
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let msg = serde_json::to_string(&msg).unwrap();
            if sender
                .send(Message::Text(Utf8Bytes::from(msg)))
                .await
                .is_err()
            {
                println!("Error sending message to {who}");
                break;
            }
        }
    });

    // This second task will receive messages from client and print them on server console
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if process_message(msg, who).is_break() {
                break;
            }
        }
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(_) => println!("send_task to {who} over"),
                Err(a) => println!("Error sending messages {a:?}")
            }
            recv_task.abort();
        },
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(_) => println!("recv_task from {who} over"),
                Err(b) => println!("Error receiving messages {b:?}")
            }
            send_task.abort();
        }
    }

    // returning from the handler closes the websocket connection
    println!("Websocket context {who} destroyed");
}

/// helper to print contents of messages to stdout. Has special treatment for Close.
fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            println!(">>> {who} sent str: {:?}", t.as_str());
        }
        Message::Binary(d) => {
            println!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {} sent close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                println!(">>> {who} somehow sent close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }
        Message::Pong(v) => {
            println!(">>> {who} sent pong with {v:?}");
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            println!(">>> {who} sent ping with {v:?}");
        }
    }
    ControlFlow::Continue(())
}
