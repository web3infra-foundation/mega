use crate::buck_controller;
use crate::model::builds;
use crate::server::AppState;
use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use dashmap::DashSet;
use futures_util::stream::{self, Stream};
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use sea_orm::sqlx::types::chrono;
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, time::Duration};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

pub fn routers() -> Router<AppState> {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    message: String,
}

static BUILDING: Lazy<DashSet<String>> = Lazy::new(DashSet::new);
// TODO avoid multi-task in one repo?
// #[debug_handler] // better error msg
// `Json` must be last arg, because it consumes the request body
async fn buck_build(
    State(state): State<AppState>,
    Json(req): Json<BuildRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let id_c = id;
    BUILDING.insert(id.to_string());
    tracing::info!("Start build task: {}", id);
    tokio::spawn(async move {
        let start_at = chrono::Utc::now();
        let output_path = format!("{}/{}", BUILD_LOG_DIR, id_c);
        let build_resp = match buck_controller::build(
            req.repo.clone(),
            req.target.clone(),
            req.args.unwrap_or_default(),
            output_path.clone(),
        )
        .await
        {
            Ok(status) => {
                let message = format!(
                    "Build {}",
                    if status.success() {
                        "success"
                    } else {
                        "failed"
                    }
                );
                tracing::info!("{}; Exit code: {:?}", message, status.code());
                BuildResult {
                    success: status.success(),
                    id: id_c.to_string(),
                    exit_code: status.code(),
                    message,
                }
            }
            Err(e) => {
                tracing::error!("Run buck2 failed: {}", e);
                BuildResult {
                    success: false,
                    id: id_c.to_string(),
                    exit_code: None,
                    message: e.to_string(),
                }
            }
        };

        let model = builds::ActiveModel {
            build_id: Set(id_c),
            output: Set(std::fs::read_to_string(&output_path).unwrap_or_default()),
            exit_code: Set(build_resp.exit_code),
            start_at: Set(start_at),
            end_at: Set(chrono::Utc::now()),
            repo_name: Set(req.repo),
            target: Set(req.target),
        };
        model.insert(&state.conn).await.unwrap();

        // remove log file
        // TODO on Linux, it's okay to delete a file that is being read
        //  but on Windows, it may cause `Permission denied`
        //  we can retry after every SSE read with file
        if std::fs::remove_file(&output_path).is_err() {
            tracing::warn!("Remove log file failed: {}", output_path);
        } else {
            tracing::info!("Remove log file: {}", output_path);
        }

        BUILDING.remove(&id_c.to_string()); // MUST after database insert to ensure data accessible

        // notify webhook
        if let Some(webhook) = req.webhook {
            let client = reqwest::Client::new();
            let resp = client.post(webhook.clone()).json(&build_resp).send().await;
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
        success: true, // TODO
        id: id.to_string(),
        exit_code: None,
        message: "Build started".to_string(),
    })
}

/// SSE
async fn build_output(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> // impl IntoResponse
{
    if !BUILDING.contains(&id) {
        // build end, no file, in database
        let build_id: Uuid = id.parse().expect("Invalid build id");
        let output = builds::Model::get_by_build_id(build_id, state.conn).await;
        let output = match output {
            Some(model) => model.output,
            None => {
                let msg = format!("Build task not found in db: {}", id);
                tracing::error!(msg);
                msg
            }
        };
        return Sse::new(stream::once(async { Ok(Event::default().data(output)) }).boxed());
    }

    // building, read from file
    let path = format!("{}/{}", BUILD_LOG_DIR, id);
    if !std::path::Path::new(&path).exists() {
        // 2 return types must same, which is hard without `.boxed()`
        // `Sse<Unfold<Reader<File>, ..., ...>>` != Sse<Once<..., ..., ...>> != Sse<Unfold<bool, ..., ...>>
        return Sse::new(
            stream::once(async { Ok(Event::default().data("Build task not found")) }).boxed(),
        );
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
