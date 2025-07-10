use callisto::mega_issue;
use jupiter::storage::stg_common::model::{ItemDetails, ItemKind};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::{label::LabelItem, mr::MegaConversation};

pub mod issue_router;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ItemRes {
    pub id: i64,
    pub link: String,
    pub title: String,
    pub status: String,
    pub author: String,
    pub open_timestamp: i64,
    pub closed_at: Option<i64>,
    pub merge_timestamp: Option<i64>,
    pub updated_at: i64,
    pub labels: Vec<LabelItem>,
    pub assignees: Vec<String>,
    pub comment_num: usize,
}

impl From<ItemDetails> for ItemRes {
    fn from(value: ItemDetails) -> Self {
        match value.item {
            ItemKind::Issue(model) => Self {
                id: model.id,
                link: model.link,
                title: model.title,
                status: model.status.to_string(),
                author: model.author,
                open_timestamp: model.created_at.and_utc().timestamp(),
                merge_timestamp: None,
                closed_at: model.closed_at.map(|dt| dt.and_utc().timestamp()),
                updated_at: model.updated_at.and_utc().timestamp(),
                labels: value.labels.into_iter().map(|m| m.into()).collect(),
                assignees: value.assignees,
                comment_num: value.comment_num,
            },
            ItemKind::Mr(model) => Self {
                id: model.id,
                link: model.link,
                title: model.title,
                status: format!("{:?}", model.status),
                author: String::new(),
                open_timestamp: model.created_at.and_utc().timestamp(),
                merge_timestamp: model.merge_date.map(|dt| dt.and_utc().timestamp()),
                closed_at: None,
                updated_at: model.updated_at.and_utc().timestamp(),
                labels: value.labels.into_iter().map(|m| m.into()).collect(),
                assignees: value.assignees,
                comment_num: value.comment_num,
            },
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct NewIssue {
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct IssueDetail {
    pub id: i64,
    pub link: String,
    pub title: String,
    pub status: String,
    pub open_timestamp: i64,
    pub conversations: Vec<MegaConversation>,
}

impl From<mega_issue::Model> for IssueDetail {
    fn from(value: mega_issue::Model) -> Self {
        Self {
            id: value.id,
            link: value.link,
            title: value.title,
            status: value.status.to_string(),
            open_timestamp: value.created_at.and_utc().timestamp(),
            conversations: vec![],
        }
    }
}
