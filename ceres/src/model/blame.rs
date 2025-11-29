//! Blame API models

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// A continuous block of lines attributed to the same commit.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlameBlock {
    pub content: String,
    pub blame_info: BlameInfo,
    pub start_line: usize,
    pub end_line: usize,
    pub line_count: usize,
}

/// Contributor information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Contributor {
    pub email: String,
    pub username: Option<String>,
    pub last_commit_time: i64,
    pub total_lines: usize,
}

/// Query parameters for blame requests
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlameQuery {
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

fn default_path() -> String {
    "/".to_string()
}

/// Request parameters for blame API endpoints
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct BlameRequest {
    #[serde(default)]
    pub refs: String,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default)]
    pub start_line: Option<usize>,
    #[serde(default)]
    pub end_line: Option<usize>,
    #[serde(default)]
    pub page: Option<usize>,
    #[serde(default)]
    pub page_size: Option<usize>,
}

impl From<&BlameRequest> for BlameQuery {
    fn from(req: &BlameRequest) -> Self {
        Self {
            start_line: req.start_line,
            end_line: req.end_line,
            page: req.page,
            page_size: req.page_size,
        }
    }
}

/// Complete blame result for a file
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BlameResult {
    pub file_path: String,
    pub blocks: Vec<BlameBlock>,
    pub total_lines: usize,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub earliest_commit_time: i64,
    pub latest_commit_time: i64,
    pub contributors: Vec<Contributor>,
}

/// Blame information for a specific commit
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlameInfo {
    pub commit_hash: String,
    pub commit_short_id: String,
    pub author_email: String,
    pub commit_time: i64,
    pub commit_message: String,
    pub commit_summary: String,
    pub original_line_number: usize,
    pub author_username: Option<String>,
    pub commit_detail_url: String,
}
