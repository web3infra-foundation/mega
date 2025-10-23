//! Blame API models for ceres
//!
//! This module contains API layer models for Git blame functionality,
//! including OpenAPI annotations for documentation generation.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// Represents a continuous block of lines attributed to the same commit.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlameBlock {
    pub content: String,
    pub blame_info: BlameInfo,
    pub start_line: usize,
    pub end_line: usize,
    pub line_count: usize,
}

impl From<jupiter::model::blame_dto::BlameBlock> for BlameBlock {
    fn from(dto: jupiter::model::blame_dto::BlameBlock) -> Self {
        Self {
            content: dto.content,
            blame_info: dto.blame_info.into(),
            start_line: dto.start_line,
            end_line: dto.end_line,
            line_count: dto.line_count,
        }
    }
}

/// Contributor information including campsite username
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Contributor {
    pub email: String,
    pub username: Option<String>,
    pub last_commit_time: i64,
    pub total_lines: usize,
}

impl From<jupiter::model::blame_dto::Contributor> for Contributor {
    fn from(dto: jupiter::model::blame_dto::Contributor) -> Self {
        Self {
            email: dto.email,
            username: dto.username,
            last_commit_time: dto.last_commit_time,
            total_lines: dto.total_lines,
        }
    }
}

/// Query parameters for blame requests
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlameQuery {
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

impl From<BlameQuery> for jupiter::model::blame_dto::BlameQuery {
    fn from(api: BlameQuery) -> Self {
        Self {
            start_line: api.start_line,
            end_line: api.end_line,
            page: api.page,
            page_size: api.page_size,
        }
    }
}

impl From<jupiter::model::blame_dto::BlameQuery> for BlameQuery {
    fn from(dto: jupiter::model::blame_dto::BlameQuery) -> Self {
        Self {
            start_line: dto.start_line,
            end_line: dto.end_line,
            page: dto.page,
            page_size: dto.page_size,
        }
    }
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
    /// Earliest commit time across all lines in the file (Unix timestamp)
    pub earliest_commit_time: i64,
    /// Latest commit time across all lines in the file (Unix timestamp)
    pub latest_commit_time: i64,
    /// List of contributors to this file
    pub contributors: Vec<Contributor>,
}

impl From<jupiter::model::blame_dto::BlameResult> for BlameResult {
    fn from(dto: jupiter::model::blame_dto::BlameResult) -> Self {
        Self {
            file_path: dto.file_path,
            blocks: dto.blocks.into_iter().map(|block| block.into()).collect(),
            total_lines: dto.total_lines,
            page: dto.page,
            page_size: dto.page_size,
            earliest_commit_time: dto.earliest_commit_time,
            latest_commit_time: dto.latest_commit_time,
            contributors: dto
                .contributors
                .into_iter()
                .map(|contributor| contributor.into())
                .collect(),
        }
    }
}

/// Blame information for a specific commit
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlameInfo {
    pub commit_hash: String,
    pub commit_short_id: String,
    pub author_email: String,
    pub author_time: i64,
    pub committer_email: String,
    pub committer_time: i64,
    pub commit_message: String,
    pub commit_summary: String,
    pub original_line_number: usize,
    // Campsite username fields for frontend to query user info via other APIs
    pub author_username: Option<String>,
    pub committer_username: Option<String>,
    pub commit_detail_url: String,
}

impl From<jupiter::model::blame_dto::BlameInfo> for BlameInfo {
    fn from(dto: jupiter::model::blame_dto::BlameInfo) -> Self {
        Self {
            commit_hash: dto.commit_hash,
            commit_short_id: dto.commit_short_id,
            author_email: dto.author_email,
            author_time: dto.author_time,
            committer_email: dto.committer_email,
            committer_time: dto.committer_time,
            commit_message: dto.commit_message,
            commit_summary: dto.commit_summary,
            original_line_number: dto.original_line_number,
            author_username: dto.author_username,
            committer_username: dto.committer_username,
            commit_detail_url: dto.commit_detail_url,
        }
    }
}
