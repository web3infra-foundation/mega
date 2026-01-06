use crate::ws::WSMessage;
use crate::{buck_controller, repo::sapling::status::Status};
use serde::Serialize;
use td_util_buck::types::ProjectRelativePath;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

/// Parameters required to execute a buck build operation.
#[derive(Debug)]
pub struct BuildRequest {
    /// Repository path or identifier
    pub repo: String,
    /// Change List identifier for context
    pub cl: String,
    /// Commit changes
    pub changes: Vec<Status<ProjectRelativePath>>,
    /// Optional explicit buck2 target label to build; when absent, worker will resolve targets from changes.
    pub target: Option<String>,
}

/// Result of a build operation containing status and metadata.
#[derive(Debug, Serialize)]
pub struct BuildResult {
    /// Whether the build operation was successful
    pub success: bool,
    /// Unique identifier for the build task
    pub id: String,
    /// Process exit code (None if not yet completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Human-readable status or error message
    pub message: String,
}

/// Initiates an asynchronous buck build process.
///
/// The build executes in a background task, allowing this function to return immediately
/// with an acknowledgment. Build progress and completion are communicated via WebSocket.
///
/// # Arguments
/// * `id` - Unique identifier for tracking the build task
/// * `req` - Build parameters including repository, target, and arguments
/// * `sender` - Channel for sending WebSocket messages during build execution
///
/// # Returns
/// Immediate acknowledgment that the build task has been queued and started
pub async fn buck_build(
    id: Uuid,
    req: BuildRequest,
    sender: UnboundedSender<WSMessage>,
) -> BuildResult {
    let id_str = id.to_string();
    tracing::info!("[Task {}] Received build request.", id_str);

    // Spawn background task to handle the actual build process
    tokio::spawn(async move {
        // Execute the build operation via buck_controller
        let build_result = match buck_controller::build(
            id_str.clone(),
            req.repo,
            req.cl,
            req.target,
            sender.clone(),
            req.changes,
        )
        .await
        {
            Ok(status) => {
                let message = format!(
                    "Build {}",
                    if status.success() {
                        "succeeded"
                    } else {
                        "failed"
                    }
                );
                tracing::info!(
                    "[Task {}] {}; Exit code: {:?}",
                    id_str,
                    message,
                    status.code()
                );
                BuildResult {
                    success: status.success(),
                    id: id_str.clone(),
                    exit_code: status.code(),
                    message,
                }
            }
            Err(e) => {
                let error_msg = format!("Build execution failed: {e}");
                tracing::error!("[Task {}] {}", id_str, error_msg);
                BuildResult {
                    success: false,
                    id: id_str.clone(),
                    exit_code: None,
                    message: error_msg,
                }
            }
        };

        // Send build completion notification via WebSocket
        let complete_msg = WSMessage::BuildComplete {
            id: build_result.id,
            success: build_result.success,
            exit_code: build_result.exit_code,
            message: build_result.message,
        };

        if sender.send(complete_msg).is_err() {
            tracing::error!(
                "[Task {}] Failed to send BuildComplete message. Connection likely lost.",
                id_str
            );
        }
    });

    // Return immediate acknowledgment of task acceptance
    // WARN: exit_code and can_auto_retry is invalid data
    BuildResult {
        success: true,
        id: id.to_string(),
        exit_code: None,
        message: "Build task has been accepted and started.".to_string(),
    }
}
