//! Buck2 build API models.
//!
//! This module defines the request/response types used between
//! Orion-Server and Monorepo during a build.
//!
//! ## Design notes
//! - Types here are **pure data models**
//! - No Buck2 execution logic should live in this module
//! - Used by both server and mono crates only

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::buck2::{status::Status, types::ProjectRelativePath};

/// Parameters required to build a task.
#[allow(dead_code)]
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct TaskBuildRequest {
    /// The repository base path
    pub repo: String,
    /// The change list link (URL)
    pub cl_link: String,
    //TODO: for old database only, delete after updated
    pub cl_id: i64,
    /// The list of file diff changes
    pub changes: Vec<Status<ProjectRelativePath>>,
    /// Buck2 target path (e.g. //app:server). Optional for backward compatibility.
    #[serde(default, alias = "targets_path")]
    pub targets: Option<Vec<String>>,
}

impl TaskBuildRequest {
    pub fn targets(&self) -> Vec<String> {
        self.targets.clone().unwrap_or_default()
    }
}

/// Request structure for Retry a build
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RetryBuildRequest {
    pub build_id: String,
    pub cl_link: String,
    pub cl_id: i64,
    pub changes: Vec<Status<ProjectRelativePath>>,
    pub targets: Option<Vec<String>>,
}

/// Result of a task build operation containing status and metadata. Used by Orion-Server
#[allow(dead_code)]
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct OrionBuildResult {
    /// Unique identifier for the build task
    pub build_id: String,
    /// Current status of the build (e.g., "queued", "running", "success", "failure")
    pub status: String,
    /// Human-readable status or error message
    pub message: String,
}

/// Response structure for build-related API endpoints in Orion-Server
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct OrionServerResponse {
    pub task_id: String,
    pub results: Vec<OrionBuildResult>,
}
