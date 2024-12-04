use axum::{Json, Router};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use serde::Deserialize;
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
}

async fn buck_build(Json(request): Json<BuildRequest>) -> impl IntoResponse {
    let repo = request.repo;
    let target = request.target;
    let args = request.args.unwrap_or_default();
    tokio::task::spawn_blocking(move || {
        match buck_controller::build(repo, target, args) {
            Ok(output) => {
                tracing::info!("Build success: {}", output);
            }
            Err(e) => {
                tracing::error!("Build failed: {}", e);
            }
        }
    });
    StatusCode::OK
}