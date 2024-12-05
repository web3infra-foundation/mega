use axum::{Json, Router};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use serde::{Deserialize, Serialize};
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
struct BuildResponse {
    success: bool,
    message: String,
}

async fn buck_build(Json(request): Json<BuildRequest>) -> impl IntoResponse {
    let repo = request.repo;
    let target = request.target;
    let args = request.args.unwrap_or_default();
    // TODO: async, notify by callback http api with build id
    tokio::task::spawn_blocking(move || {
        let build_resp = match buck_controller::build(repo, target, args) {
            Ok(output) => {
                tracing::info!("Build success: {}", output);
                BuildResponse {
                    success: true,
                    message: output,
                }
            }
            Err(e) => {
                tracing::error!("Build failed: {}", e);
                BuildResponse {
                    success: false,
                    message: e.to_string(),
                }
            }
        };

        // notify webhook
        if let Some(webhook) = request.webhook {
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
    StatusCode::OK
}