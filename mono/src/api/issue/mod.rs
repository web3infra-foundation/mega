use callisto::mega_issue;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::{label::LabelItem, mr::MegaConversation};

pub mod issue_router;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct IssueItem {
    pub link: String,
    pub title: String,
    pub status: String,
    pub user_id: String,
    pub labels: Vec<LabelItem>,
    pub open_timestamp: i64,
    pub closed_at: Option<i64>,
    pub updated_at: i64,
}

impl From<mega_issue::Model> for IssueItem {
    fn from(value: mega_issue::Model) -> Self {
        Self {
            link: value.link,
            title: value.title,
            status: value.status.to_string(),
            user_id: value.user_id,
            open_timestamp: value.created_at.and_utc().timestamp(),
            closed_at: value.closed_at.map(|dt| dt.and_utc().timestamp()),
            updated_at: value.updated_at.and_utc().timestamp(),
            labels: vec![],
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

#[derive(Deserialize, ToSchema)]
pub struct LabelUpdatePayload {
    label_ids: Vec<i64>,
    item_id: i64,
    link: String,
}
