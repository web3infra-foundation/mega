use api_model::common::CommonPage;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::model::change_list::DiffItemSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, Default)]
#[serde(rename_all = "PascalCase")]
pub enum GpgStatus {
    Verified,
    Unverified,
    #[default]
    NoSignature,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct CommitSummary {
    pub sha: String,
    pub short_message: String,
    pub author: String,
    pub committer: String,
    pub date: String,
    pub parents: Vec<String>,
    /// GPG verification status for this commit.
    #[serde(default)]
    pub gpg_status: GpgStatus,
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
pub struct CommitFilesChangedPage {
    pub commit: CommitSummary,
    pub page: CommonPage<DiffItemSchema>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommitBindingResponse {
    pub username: Option<String>,
}
