//! Types related to Buck2 build system.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request from server to worker to build a target.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum Status<Path> {
    Modified(Path),
    Added(Path),
    Removed(Path),
}

/// Task phase when in buck2 build
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum TaskPhase {
    DownloadingSource,
    RunningBuild,
}

/// Represents a file path relative to the project root.
#[allow(dead_code)]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProjectRelativePath(String);

impl ProjectRelativePath {
    /// Creates a new ProjectRelativePath from a string slice.
    pub fn new(path: &str) -> Self {
        Self(path.to_owned())
    }

    /// Attempts to create a ProjectRelativePath from an absolute path and a base path.
    pub fn from_abs(abs_path: &str, base: &str) -> Option<Self> {
        let opt = abs_path
            .strip_prefix(base)
            .map(|s| s.trim_start_matches("/"));
        opt.map(|s| Self(s.to_owned()))
    }
}

/// Supported read modes for log APIs.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogReadMode {
    #[default]
    Full,
    Segment,
}

/// Log stream event emitted by Orion worker/build processing.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogEvent {
    pub task_id: String,
    pub repo_name: String,
    pub build_id: String,
    pub line: String,
    pub is_end: bool,
}

/// Log segment read result.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogSegment {
    pub build_id: String,
    pub offset: u64,
    pub len: usize,
    pub data: String,
    pub next_offset: u64,
    pub file_size: u64,
    /// Whether we reached end of file
    pub eof: bool,
}

/// Query parameters for target log APIs.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TargetLogQuery {
    #[serde(default)]
    pub r#type: LogReadMode,
    pub build_id: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

/// Query parameters for task history log APIs.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TaskHistoryQuery {
    pub task_id: String,
    pub build_id: String,
    pub repo: String,
    pub start: Option<usize>,
    pub end: Option<usize>,
}

/// Log lines response for history reads.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogLinesResponse {
    pub data: Vec<String>,
    pub len: usize,
}

/// Log lines response for target reads.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TargetLogLinesResponse {
    pub data: Vec<String>,
    pub len: usize,
    pub build_id: String,
}

/// Error response for log-related APIs.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogErrorResponse {
    pub message: String,
}
