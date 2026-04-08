use std::{any::Any, panic::AssertUnwindSafe};

use api_model::buck2::{status::Status, types::ProjectRelativePath, ws::WSMessage};
use futures_util::FutureExt;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use crate::buck_controller;

// /// Parameters required to execute a buck build operation.
// #[derive(Debug)]
// pub struct BuildRequest {
//     /// Monorepo mount path (Buck2 project root or subdirectory)
//     pub repo: String,
//     /// Change List identifier for context
//     pub cl: String,
//     /// Commit changes
//     pub changes: Vec<Status<ProjectRelativePath>>,
// }

/// Result of a build operation containing status and metadata.
#[derive(Debug, Serialize)]
pub struct BuildResult {
    /// Whether the build operation was successful
    pub success: bool,
    /// Unique identifier for the build task
    pub build_id: String,
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
    cl_link: String,
    repo: String,
    changes: Vec<Status<ProjectRelativePath>>,
    sender: UnboundedSender<WSMessage>,
) -> BuildResult {
    let id_str = id.to_string();
    tracing::info!("[Task {}] Received build request.", id_str);

    // Spawn background task to handle the actual build process
    tokio::spawn(async move {
        // Execute the build operation via buck_controller
        let guarded = AssertUnwindSafe(buck_controller::build(
            id_str.clone(),
            repo,
            cl_link,
            sender.clone(),
            changes,
        ))
        .catch_unwind()
        .await;
        let build_result = match guarded {
            Ok(Ok(status)) => {
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
                    build_id: id_str.clone(),
                    exit_code: status.code(),
                    message,
                }
            }
            Ok(Err(e)) => {
                let error_msg = format!("Build execution failed: {e}");
                tracing::error!("[Task {}] {}", id_str, error_msg);
                BuildResult {
                    success: false,
                    build_id: id_str.clone(),
                    exit_code: None,
                    message: error_msg,
                }
            }
            Err(panic_payload) => {
                let panic_msg = panic_payload_to_string(panic_payload.as_ref());
                let error_msg = format!("Build execution panicked: {panic_msg}");
                tracing::error!("[Task {}] {}", id_str, error_msg);
                BuildResult {
                    success: false,
                    build_id: id_str.clone(),
                    exit_code: None,
                    message: error_msg,
                }
            }
        };

        // Send build completion notification via WebSocket
        let complete_msg = WSMessage::TaskBuildComplete {
            build_id: build_result.build_id,
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
        build_id: id.to_string(),
        exit_code: None,
        message: "Build task has been accepted and started.".to_string(),
    }
}

fn panic_payload_to_string(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "non-string panic payload".to_string()
}

#[cfg(test)]
mod tests {
    use super::panic_payload_to_string;

    #[test]
    fn test_panic_payload_to_string_handles_common_payloads() {
        assert_eq!(panic_payload_to_string(&"panic text"), "panic text");
        assert_eq!(
            panic_payload_to_string(&"panic string".to_string()),
            "panic string"
        );
    }
}
