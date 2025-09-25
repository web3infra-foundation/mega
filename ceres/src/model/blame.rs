//! Blame API models for ceres
//!
//! This module contains API layer models for Git blame functionality,
//! including OpenAPI annotations for documentation generation.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

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

/// Request parameters for blame API endpoints
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct BlameRequest {
    #[serde(default)]
    pub refs: String,
    #[serde(default)]
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
    pub lines: Vec<BlameLine>,
    pub total_lines: usize,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

impl From<jupiter::model::blame_dto::BlameResult> for BlameResult {
    fn from(dto: jupiter::model::blame_dto::BlameResult) -> Self {
        Self {
            file_path: dto.file_path,
            lines: dto.lines.into_iter().map(|line| line.into()).collect(),
            total_lines: dto.total_lines,
            page: dto.page,
            page_size: dto.page_size,
        }
    }
}

/// A single line with its blame information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlameLine {
    pub line_number: usize,
    pub content: String,
    pub blame_info: BlameInfo,
}

impl From<jupiter::model::blame_dto::BlameLine> for BlameLine {
    fn from(dto: jupiter::model::blame_dto::BlameLine) -> Self {
        Self {
            line_number: dto.line_number,
            content: dto.content,
            blame_info: dto.blame_info.into(),
        }
    }
}

/// Blame information for a specific commit
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlameInfo {
    pub commit_hash: String,
    pub commit_short_id: String,
    pub author_name: String,
    pub author_email: String,
    pub author_time: i64,
    pub committer_name: String,
    pub committer_email: String,
    pub committer_time: i64,
    pub commit_message: String,
    pub commit_summary: String,
    pub original_line_number: usize,
    // URL fields for frontend navigation
    pub author_avatar_url: String,
    pub commit_detail_url: String,
    pub author_profile_url: String,
}

impl From<jupiter::model::blame_dto::BlameInfo> for BlameInfo {
    fn from(dto: jupiter::model::blame_dto::BlameInfo) -> Self {
        Self {
            commit_hash: dto.commit_hash,
            commit_short_id: dto.commit_short_id,
            author_name: dto.author_name,
            author_email: dto.author_email,
            author_time: dto.author_time,
            committer_name: dto.committer_name,
            committer_email: dto.committer_email,
            committer_time: dto.committer_time,
            commit_message: dto.commit_message,
            commit_summary: dto.commit_summary,
            original_line_number: dto.original_line_number,
            author_avatar_url: dto.author_avatar_url,
            commit_detail_url: dto.commit_detail_url,
            author_profile_url: dto.author_profile_url,
        }
    }
}
