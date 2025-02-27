use serde::{Deserialize, Serialize};

use callisto::{mega_conversation, mega_mr};

pub mod mr_router;

#[derive(Deserialize)]
pub struct MRStatusParams {
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct MrInfoItem {
    pub link: String,
    pub title: String,
    pub status: String,
    pub open_timestamp: i64,
    pub merge_timestamp: Option<i64>,
    pub updated_at: i64,
}

impl From<mega_mr::Model> for MrInfoItem {
    fn from(value: mega_mr::Model) -> Self {
        Self {
            link: value.link,
            title: value.title,
            status: value.status.to_string(),
            open_timestamp: value.created_at.and_utc().timestamp(),
            merge_timestamp: value.merge_date.map(|dt| dt.and_utc().timestamp()),
            updated_at: value.updated_at.and_utc().timestamp(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MRDetail {
    pub id: i64,
    pub link: String,
    pub title: String,
    pub status: String,
    pub open_timestamp: i64,
    pub merge_timestamp: Option<i64>,
    pub conversations: Vec<MegaConversation>,
}

impl From<mega_mr::Model> for MRDetail {
    fn from(value: mega_mr::Model) -> Self {
        Self {
            id: value.id,
            link: value.link,
            title: value.title,
            status: value.status.to_string(),
            open_timestamp: value.created_at.and_utc().timestamp(),
            merge_timestamp: value.merge_date.map(|dt| dt.and_utc().timestamp()),
            conversations: vec![],
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MegaConversation {
    pub id: i64,
    pub user_id: i64,
    pub conv_type: String,
    pub comment: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<mega_conversation::Model> for MegaConversation {
    fn from(value: mega_conversation::Model) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            conv_type: value.conv_type.to_string(),
            comment: value.comment,
            created_at: value.created_at.and_utc().timestamp(),
            updated_at: value.updated_at.and_utc().timestamp(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct FilesChangedItem {
    pub path: String,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct FilesChangedList {
    pub files: Vec<FilesChangedItem>,
    pub content: String,
}
