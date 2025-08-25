use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatusEnum {
    #[serde(rename = "syncing")]
    Syncing,
    #[serde(rename = "succeed")]
    Succeed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "analysing")]
    Analysing,
    #[serde(rename = "analysed")]
    Analysed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrateTypeEnum {
    #[serde(rename = "lib")]
    Lib,
    #[serde(rename = "application")]
    Application,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKindEnum {
    #[serde(rename = "mega")]
    Mega,
    #[serde(rename = "user")]
    User,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceOfDataEnum {
    #[serde(rename = "cratesio")]
    Cratesio,
    #[serde(rename = "github")]
    Github,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoSyncModel {
    pub id: i64,
    pub crate_name: String,
    pub github_url: Option<String>,
    pub mega_url: String,
    pub crate_type: CrateTypeEnum,
    pub status: SyncStatusEnum,
    pub err_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageModel {
    pub db_model: RepoSyncModel,
    pub message_kind: MessageKindEnum,
    pub source_of_data: SourceOfDataEnum,
    pub timestamp: DateTime<Utc>,
    pub extra_field: Option<String>,
}
