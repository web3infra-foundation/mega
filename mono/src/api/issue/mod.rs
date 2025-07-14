use jupiter::{
    model::common::{ItemDetails, ItemKind},
    model::issue_dto::IssueDetails,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::{conversation::ConversationItem, label::LabelItem};

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

#[derive(Serialize, ToSchema)]
pub struct IssueDetailRes {
    pub id: i64,
    pub link: String,
    pub title: String,
    pub status: String,
    pub open_timestamp: i64,
    pub conversations: Vec<ConversationItem>,
    pub labels: Vec<LabelItem>,
    pub assignees: Vec<String>,
}

impl From<IssueDetails> for IssueDetailRes {
    fn from(value: IssueDetails) -> Self {
        Self {
            id: value.issue.id,
            link: value.issue.link,
            title: value.issue.title,
            status: value.issue.status.to_string(),
            open_timestamp: value.issue.created_at.and_utc().timestamp(),
            conversations: value
                .conversations
                .into_iter()
                .map(|x| ConversationItem::from_model(x.conversation, x.reactions, &value.username))
                .collect(),
            labels: value.labels.into_iter().map(|x| x.into()).collect(),
            assignees: value
                .assignees
                .into_iter()
                .map(|x| x.assignnee_id)
                .collect(),
        }
    }
}
