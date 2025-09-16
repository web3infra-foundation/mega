use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub crate_name: String,
    pub github_url: Option<String>,
    pub mega_url: String,
    pub crate_type: CrateType,
    pub status: RepoSyncStatus,
    pub err_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum CrateType {
    Lib,
    Application,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum RepoSyncStatus {
    Syncing,
    Succeed,
    Failed,
    Analysing,
    Analysed,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum MessageKind {
    Mega,
    User,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceOfData {
    Cratesio,
    Github,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageModel {
    pub db_model: Model, // Wraps the database Model
    pub message_kind: MessageKind,
    pub source_of_data: SourceOfData,
    pub timestamp: DateTime<Utc>, // Timestamp when the message is sent
    pub extra_field: Option<String>,      // Additional fields can be added
}
