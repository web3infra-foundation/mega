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
