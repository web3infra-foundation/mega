use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubRepoMessage {
    pub repo_name: String,
    pub github_url: String,
    pub mega_url: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uuid: String,
}
