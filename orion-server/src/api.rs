use crate::model::builds;
use axum::body::Bytes;
use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use axum::extract::{ConnectInfo, Path, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Sse};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use axum_extra::json;
use dashmap::DashMap;
use futures_util::{SinkExt, Stream, StreamExt, stream};
use once_cell::sync::Lazy;
use orion::ws::WSMessage;
use rand::seq::SliceRandom;
use scopeguard::defer;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::DateTimeUtc;
use sea_orm::sqlx::types::chrono;
use sea_orm::{ActiveModelTrait, DatabaseConnection};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::io::Write;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

static BUILD_LOG_DIR: Lazy<String> =
    Lazy::new(|| std::env::var("BUILD_LOG_DIR").expect("BUILD_LOG_DIR must be set"));

#[derive(Debug, Deserialize)]
struct BuildRequest {
    repo: String,
    target: String,
    args: Option<Vec<String>>,
}

pub struct BuildInfo {
    repo: String,
    target: String,
    args: Option<Vec<String>>,
    start_at: DateTimeUtc,
}

#[derive(Debug, Serialize, Default)]
enum TaskStatusEnum {
    Building,
    Interrupted, // exit code is None
    Failed,
    Completed,
    #[default]
    NotFound,
}

#[derive(Debug, Serialize, Default)]
struct TaskStatus {
    status: TaskStatusEnum,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub clients: Arc<DashMap<String, UnboundedSender<WSMessage>>>,
    pub conn: DatabaseConnection,
    pub building: Arc<DashMap<String, BuildInfo>>,
}

pub fn routers() -> Router<AppState> {
    Router::new()
        .route("/ws", any(ws_handler))
        .route("/task", post(task_handler))
        .route("/task-status/{id}", get(task_status_handler))
        .route("/task-output/{id}", get(task_output_handler))
}

async fn task_status_handler(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let (code, status) = if state.building.contains_key(&id) {
        (
            StatusCode::OK,
            TaskStatus {
                status: TaskStatusEnum::Building,
                ..Default::default()
            },
        )
    } else {
        match Uuid::parse_str(&id) {
            Ok(id) => {
                let output = builds::Model::get_by_build_id(id, state.conn).await;
                match output {
                    Some(model) => {
                        let status = if model.exit_code.is_none() {
                            TaskStatusEnum::Interrupted
                        } else if model.exit_code.unwrap() == 0 {
                            TaskStatusEnum::Completed
                        } else {
                            TaskStatusEnum::Failed
                        };
                        (
                            StatusCode::OK,
                            TaskStatus {
                                status,
                                exit_code: model.exit_code,
                                ..Default::default()
                            },
                        )
                    }
                    None => (
                        StatusCode::NOT_FOUND,
                        TaskStatus {
                            status: TaskStatusEnum::NotFound,
                            message: Some("Build task not found".to_string()),
                            ..Default::default()
                        },
                    ),
                }
            }
            Err(_) => (
                StatusCode::BAD_REQUEST,
                TaskStatus {
                    status: TaskStatusEnum::NotFound,
                    message: Some("Invalid build id".to_string()),
                    ..Default::default()
                },
            ),
        }
    };
    (code, Json(status))
}

/// SSE
async fn task_output_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> // impl IntoResponse
{
    let path = format!("{}/{}", *BUILD_LOG_DIR, id);
    if !std::path::Path::new(&path).exists() {
        // 2 return types must same, which is hard without `.boxed()`
        // `Sse<Unfold<Reader<File>, ..., ...>>` != Sse<Once<..., ..., ...>> != Sse<Unfold<bool, ..., ...>>
        return Sse::new(
            stream::once(async { Ok(Event::default().data("Task output file not found")) }).boxed(),
        );
    }

    let file = tokio::fs::File::open(&path).await.unwrap(); // read-only mode
    let reader = tokio::io::BufReader::new(file);

    let stream = stream::unfold(reader, move |mut reader| {
        let id_c = id.clone(); // must, or err
        let building = state.building.clone();
        async move {
            let mut buf = String::new();
            let is_building = building.contains_key(&id_c); // MUST check before reading
            let size = reader.read_to_string(&mut buf).await.unwrap();
            if size == 0 {
                if is_building {
                    // wait for new content
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let size = reader.read_to_string(&mut buf).await.unwrap();
                    if size > 0 {
                        tracing::debug!("Read: {}", buf); // little duplicate code, but more efficient
                        Some((Ok(Event::default().data(buf)), reader))
                    } else {
                        tracing::debug!("Not Modified, waiting...");
                        // return control to `axum`, or it can't auto-detect client disconnect & close
                        Some((Ok(Event::default().comment("")), reader))
                    }
                } else {
                    // build end & no more content
                    None
                }
            } else {
                tracing::debug!("Read: {}", buf);
                Some((Ok(Event::default().data(buf)), reader))
            }
        }
    });

    Sse::new(stream.boxed()).keep_alive(KeepAlive::new()) // empty comment to keep alive
}

async fn task_handler(
    State(state): State<AppState>,
    Json(req): Json<BuildRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7().to_string();
    state.building.insert(
        id.clone(),
        BuildInfo {
            repo: req.repo.clone(),
            target: req.target.clone(),
            args: req.args.clone(),
            start_at: chrono::Utc::now(),
        },
    );

    let client_ids: Vec<_> = state
        .clients
        .iter()
        .map(|entry| entry.key().clone())
        .collect();

    if client_ids.is_empty() {
        return json!({"message": "No clients connected"});
    }

    let mut rng = rand::thread_rng();
    let chosen_id = client_ids.choose(&mut rng).unwrap();

    let msg = WSMessage::Task {
        id: id.clone(),
        repo: req.repo,
        target: req.target,
        args: req.args,
    };

    state.clients.get(chosen_id).unwrap().send(msg).unwrap(); // TODO client maybe disconnected

    json!({"task_id": id, "client_id": chosen_id})
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
    let clients = state.clients.clone();
    clients.insert(client_id.clone(), tx);
    defer! {
        println!("clean Client {who}.");
        clients.remove(&client_id);
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
            if process_message(msg, who, state.clone()).await.is_break() {
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

async fn process_message(msg: Message, who: SocketAddr, state: AppState) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => match serde_json::from_str::<WSMessage>(t.as_str()) {
            Ok(msg) => match msg {
                // todo useless ?
                WSMessage::TaskAck {
                    id,
                    success,
                    message,
                } => {
                    println!(">>> task ack: id:{id}, success:{success}, msg:{message}");
                }
                WSMessage::BuildOutput { id, output } => {
                    println!(">>> build output: id:{id}, output:{output}");
                    let mut file = std::fs::OpenOptions::new() // TODO optimize: open & close too many times
                        .append(true)
                        .create(true)
                        .open(format!("{}/{}", *BUILD_LOG_DIR, id))
                        .unwrap();
                    file.write_all(format!("{output}\n").as_bytes()).unwrap();
                }
                WSMessage::BuildComplete {
                    id,
                    success,
                    exit_code,
                    message,
                } => {
                    println!(
                        ">>> got build complete: id:{id}, success:{success}, exit_code:{exit_code:?}, msg:{message}"
                    );
                    let info = state.building.get(&id).expect("Build info not found");
                    let model = builds::ActiveModel {
                        build_id: Set(id.parse().unwrap()),
                        output_file: Set(format!("{}/{}", *BUILD_LOG_DIR, id)),
                        exit_code: Set(exit_code),
                        start_at: Set(info.start_at),
                        end_at: Set(chrono::Utc::now()),
                        repo_name: Set(info.repo.clone()),
                        target: Set(info.target.clone()),
                        arguments: Set(info.args.clone().unwrap_or_default().join(" ")),
                    };
                    drop(info); // !!release ref or deadlock when insert
                    model.insert(&state.conn).await.unwrap();
                    state.building.remove(&id); // task over
                }
                _ => unreachable!(),
            },
            Err(e) => {
                println!("Error parsing message: {e}");
            }
        },
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
