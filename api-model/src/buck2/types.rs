//! Types related to Buck2 build system.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use parse_display::Display;

/// Task phase when in buck2 build
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum TaskPhase {
    DownloadingSource,
    RunningBuild,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Display, Deserialize, Serialize, ToSchema)]
pub struct ProjectRelativePath(String);

impl ProjectRelativePath {
    pub fn new(path: &str) -> Self {
        Self(path.to_owned())
    }

    pub fn join(&self, suffix: &str) -> Self {
        if self.0.is_empty() {
            Self(suffix.to_owned())
        } else {
            Self(format!("{}/{}", self.0, suffix))
        }
    }

    /// ```
    /// use buck2::types::ProjectRelativePath;
    /// assert_eq!(
    ///     ProjectRelativePath::new("foo/bar.bzl").extension(),
    ///     Some("bzl")
    /// );
    /// assert_eq!(
    ///     ProjectRelativePath::new("foo/bar.bzl/baz").extension(),
    ///     None
    /// );
    /// assert_eq!(ProjectRelativePath::new("foo/bar/baz").extension(), None);
    /// ```
    pub fn extension(&self) -> Option<&str> {
        self.0
            .as_str()
            .rsplit_once('/')
            .unwrap_or_default()
            .1
            .rsplit_once('.')
            .map(|x| x.1)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ProjectRelativePath {
    fn as_ref(&self) -> &str {
        self.as_str()
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
