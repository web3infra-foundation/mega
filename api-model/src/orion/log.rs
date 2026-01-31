use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogSegment {
    pub build_id: String,
    pub offset: u64,
    pub len: usize,
    pub data: String,
    pub next_offset: u64,
    pub file_size: u64,
    pub eof: bool,
}

/// Query parameters for target log APIs.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TargetLogQuery {
    #[serde(default)]
    pub r#type: LogReadMode,
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
