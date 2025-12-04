use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct CommitSummary {
    pub sha: String,
    pub short_message: String,
    pub author: String,
    pub committer: String,
    pub date: String,
    pub parents: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, Default)]
pub struct CommitHistoryParams {
    /// path: dir or file path filter
    #[serde(default)]
    pub path: String,
    /// refs: branch/tag
    #[serde(default)]
    pub refs: String,
    /// author: author name filter
    #[serde(default)]
    pub author: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct CommitDetail {
    pub commit: CommitSummary,
    /// Unified diff list compared with the previous commit (or merged parent in case of multiple parents)
    pub diffs: Vec<common::model::DiffItem>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommitBindingResponse {
    pub username: Option<String>,
}
