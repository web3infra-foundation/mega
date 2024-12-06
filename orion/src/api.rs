use axum::{Json, Router};
use axum::response::IntoResponse;
use axum::routing::post;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::buck_controller;

pub fn routers() -> Router {
    Router::new()
        .route("/build", post(buck_build))
}
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

async fn buck_build(Json(req): Json<BuildRequest>) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let id_c = id.clone();
    tracing::info!("Start build task: {}", id);
    tokio::task::spawn_blocking(move || {
        let build_resp = match buck_controller::build(
            req.repo,
            req.target,
            req.args.unwrap_or_default(),
            id_c.to_string())
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