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
    //pub created_at: DateTime<Utc>,
    //pub updated_at: DateTime<Utc>,
    //pub message_kind: MessageKind,
    //pub sourc_of_data: SourceOfData,
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
    pub db_model: Model, // 包装数据库 Model
    pub message_kind: MessageKind,
    pub source_of_data: SourceOfData,
    pub timestamp: DateTime<Utc>, // 消息发送时的时间戳
    pub extra_field: Option<String>,      // 可以添加额外字段
}
