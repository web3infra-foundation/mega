use serde::{Deserialize, Serialize};

use callisto::{
    mega_conversation, mega_mr,
    sea_orm_active_enums::{ConvTypeEnum, MergeStatusEnum},
};
use utoipa::ToSchema;

pub mod mr_router;

#[derive(Deserialize, ToSchema)]
pub struct MRStatusParams {
    pub status: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MrInfoItem {
    pub link: String,
    pub title: String,
    pub status: MergeStatusEnum,
    pub open_timestamp: i64,
    pub merge_timestamp: Option<i64>,
    pub updated_at: i64,
}

impl From<mega_mr::Model> for MrInfoItem {
    fn from(value: mega_mr::Model) -> Self {
        Self {
            link: value.link,
            title: value.title,
            status: value.status,
            open_timestamp: value.created_at.and_utc().timestamp(),
            merge_timestamp: value.merge_date.map(|dt| dt.and_utc().timestamp()),
            updated_at: value.updated_at.and_utc().timestamp(),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MRDetail {
    pub id: i64,
    pub link: String,
    pub title: String,
    pub status: MergeStatusEnum,
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
            status: value.status,
            open_timestamp: value.created_at.and_utc().timestamp(),
            merge_timestamp: value.merge_date.map(|dt| dt.and_utc().timestamp()),
            conversations: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MegaConversation {
    pub id: i64,
    pub user_id: String,
    pub conv_type: ConvTypeEnum,
    pub comment: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<mega_conversation::Model> for MegaConversation {
    fn from(value: mega_conversation::Model) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            conv_type: value.conv_type,
            comment: value.comment,
            created_at: value.created_at.and_utc().timestamp(),
            updated_at: value.updated_at.and_utc().timestamp(),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct FilesChangedItem {
    pub path: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct FilesChangedList {
    pub files: Vec<FilesChangedItem>,
    pub content: String,
}

#[derive(Deserialize, ToSchema)]
pub struct SaveCommentRequest {
    pub content: String,
}
