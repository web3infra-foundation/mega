use axum::{Json, Router};
use axum::response::{IntoResponse, Sse};
use axum::response::sse::{Event, KeepAlive};
use axum::routing::{get, post};
use futures_util::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::{time::Duration, convert::Infallible};
use axum::extract::Path;
use dashmap::DashSet;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use tokio::io::AsyncReadExt;
use uuid::Uuid;
use crate::buck_controller;

pub fn routers() -> Router {
    Router::new()
        .route("/build", post(buck_build))
        .route("/build-output/:id", get(build_output))
}

const BUILD_LOG_DIR: &str = "/tmp/buck2ctl";
#[derive(Debug, Deserialize)]
struct BuildRequest {
    repo: String,
    target: String,
    args: Option<Vec<String>>,
    webhook: Option<String>, // post
}

#[derive(Debug, Serialize)]
struct BuildResult {
    success: bool,
    id: String,
    message: String,
}

static BUILDING: Lazy<DashSet<String>> = Lazy::new(|| DashSet::new());
// TODO avoid multi-task in one repo?
async fn buck_build(Json(req): Json<BuildRequest>) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let id_c = id.clone();
    BUILDING.insert(id.to_string());
    tracing::info!("Start build task: {}", id);
    tokio::task::spawn_blocking(move || {
        let build_resp = match buck_controller::build(
            req.repo,
            req.target,
            req.args.unwrap_or_default(),
            format!("{}/{}", BUILD_LOG_DIR, id_c.to_string()))
        {
            Ok(output) => {
                tracing::info!("Build success: {}", output);
                BuildResult {
                    success: true,
                    id: id_c.to_string(),
                    message: output,
                }
            }
            Err(e) => {
                tracing::error!("Build failed: {}", e);
                BuildResult {
                    success: false,
                    id: id_c.to_string(),
                    message: e.to_string(),
                }
            }
        };

        BUILDING.remove(&id_c.to_string());

        // notify webhook
        if let Some(webhook) = req.webhook {
            let client = reqwest::blocking::Client::new();
            let resp = client.post(webhook.clone())
                .json(&build_resp)
                .send();
            match resp {
                Ok(resp) => {
                    if resp.status().is_success() {
                        tracing::info!("Webhook notify success: {}", webhook);
                    } else {
                        tracing::error!("Webhook notify failed: {}", resp.status());
                    }
                }
                Err(e) => {
                    tracing::error!("Webhook notify failed: {}", e);
                }
            }
        }
    });

    Json(BuildResult {
        success: true,
        id: id.to_string(),
        message: "Build started".to_string(),
    })
}

/// SSE
async fn build_output(Path(id): Path<String>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> { // impl IntoResponse
    let path = format!("{}/{}", BUILD_LOG_DIR, id);
    if !std::path::Path::new(&path).exists() {
        // 2 return types must same, which is hard without `.boxed()`
        // `Sse<Unfold<Reader<File>, ..., ...>>` != Sse<Once<..., ..., ...>> != Sse<Unfold<bool, ..., ...>>
        return Sse::new(stream::once(async { Ok(Event::default().data("Build task not found")) }).boxed());
    }

    let file = tokio::fs::File::open(&path).await.unwrap(); // read-only mode
    let reader = tokio::io::BufReader::new(file);

    let stream = stream::unfold(reader, move |mut reader| {
        let id_c = id.clone(); // must, or err
        async move {
            let mut buf = String::new();
            let is_building = BUILDING.contains(&id_c); // MUST check before reading
            let size = reader.read_to_string(&mut buf).await.unwrap();
            if size == 0 {
                if is_building {
                    // wait for new content
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    let size = reader.read_to_string(&mut buf).await.unwrap();
                    if size > 0 {
                        tracing::debug!("Read: {}", buf); // little duplicate code, but more efficient
                        Some((Ok::<Event, Infallible>(Event::default().data(buf)), reader))
                    } else {
                        tracing::debug!("Not Modified, waiting...");
                        // return control to `axum`, or it can't auto-detect client disconnect & close
                        Some((Ok::<Event, Infallible>(Event::default().comment("")), reader))
                    }
                } else {
                    // build end & no more content
                    None
                }
            } else {
                tracing::debug!("Read: {}", buf);
                Some((Ok::<Event, Infallible>(Event::default().data(buf)), reader))
            }
        }
    });

    Sse::new(stream.boxed()).keep_alive(KeepAlive::new()) // empty comment to keep alive
}