use axum::Json;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::ops::ControlFlow;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{mpsc, OnceCell};
// we will use tungstenite for websocket client impl (same library as what axum is using)
use crate::api::{buck_build, BuildRequest};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tungstenite::Utf8Bytes;

static SENDER: OnceCell<UnboundedSender<WSMessage>> = OnceCell::const_new();

#[derive(Debug, Serialize, Deserialize)]
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
}

pub async fn spawn_client(server: &str) {
    let ws_stream = match connect_async(server).await {
        Ok((stream, response)) => {
            println!("Server response was {response:?}");
            stream
        }
        Err(e) => {
            println!("WebSocket handshake failed with {e}!");
            return;
        }
    };

    let (mut sender, mut receiver) = ws_stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<WSMessage>();
    SENDER.set(tx).unwrap();

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let msg = serde_json::to_string(&msg).unwrap();
            if let Err(e) = sender.send(Message::Text(Utf8Bytes::from(msg))).await {
                println!("Error sending message: {e}");
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if process_message(msg).await.is_break() {
                break;
            }
        }
    });

    //wait for either task to finish and kill the other task
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }
}

async fn process_message(msg: Message) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => match serde_json::from_str::<WSMessage>(&t) {
            Ok(msg) => match msg {
                WSMessage::Task {
                    id,
                    repo,
                    target,
                    args,
                } => {
                    println!(">>> got task: id:{id}, repo:{repo}, target:{target}, args:{args:?}");
                    let Json(res) = buck_build(
                        id.parse().unwrap(),
                        BuildRequest {
                            repo,
                            target,
                            args,
                            webhook: None,
                        },
                        SENDER.get().unwrap().clone(),
                    )
                    .await;
                    SENDER
                        .get()
                        .unwrap()
                        .send(WSMessage::TaskAck {
                            id: id.clone(),
                            success: res.success,
                            message: res.message,
                        })
                        .unwrap();
                }
                _ => {
                    unreachable!("Impossible msg to client {msg:?}");
                }
            },
            Err(e) => {
                println!("Error parsing message: {e}");
            }
        },
        Message::Binary(d) => {
            println!(">>> got {} bytes: {:?}", d.len(), d);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> got close with code {} and reason `{}`",
                    cf.code, cf.reason
                );
            } else {
                println!(">>> somehow got close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }

        Message::Pong(v) => {
            println!(">>> got pong with {v:?}");
        }
        // Just as with axum server, the underlying tungstenite websocket library
        // will handle Ping for you automagically by replying with Pong and copying the
        // v according to spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            println!(">>> got ping with {v:?}");
        }

        Message::Frame(_) => {
            unreachable!("This is never supposed to happen")
        }
    }
    ControlFlow::Continue(())
}
