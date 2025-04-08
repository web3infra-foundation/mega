use crate::buck_controller;
use crate::ws::WSMessage;
use axum::Json;
use dashmap::DashSet;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;
#[derive(Debug, Deserialize)]
pub struct BuildRequest {
    pub repo: String,
    pub target: String,
    pub args: Option<Vec<String>>,
    // pub webhook: Option<String>, // post
}

#[derive(Debug, Serialize)]
pub struct BuildResult {
    pub success: bool,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub message: String,
}

static BUILDING: Lazy<DashSet<String>> = Lazy::new(DashSet::new);
// TODO avoid multi-task in one repo?
// #[debug_handler] // better error msg
// `Json` must be last arg, because it consumes the request body
pub async fn buck_build(
    id: Uuid,
    req: BuildRequest,
    sender: UnboundedSender<WSMessage>,
) -> Json<BuildResult> {
    let id_c = id;
    BUILDING.insert(id.to_string());
    tracing::info!("Start build task: {}", id);
    tokio::spawn(async move {
        let build_resp = match buck_controller::build(
            id_c.to_string(),
            req.repo.clone(),
            req.target.clone(),
            req.args.unwrap_or_default(),
            sender.clone(),
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
        BUILDING.remove(&id_c.to_string()); // MUST after database insert to ensure data accessible

        sender
            .send(WSMessage::BuildComplete {
                id: id_c.to_string(),
                success: build_resp.success,
                exit_code: build_resp.exit_code,
                message: build_resp.message.clone(),
            })
            .unwrap();
    });

    Json(BuildResult {
        success: true, // TODO
        id: id.to_string(),
        exit_code: None,
        message: "Build started".to_string(),
    })
}
