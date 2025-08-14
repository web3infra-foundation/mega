use std::str::FromStr;

use ceres::model::mr::MrDiffFile;
use jupiter::model::mr_dto::MRDetails;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::{conversation::ConversationItem, label::LabelItem};
use callisto::sea_orm_active_enums::MergeStatusEnum;
use common::model::CommonPage;
use neptune::model::diff_model::DiffItem;

pub mod mr_router;

#[derive(Serialize, ToSchema)]
pub struct MRDetailRes {
    pub id: i64,
    pub link: String,
    pub title: String,
    pub status: MergeStatus,
    pub open_timestamp: i64,
    pub merge_timestamp: Option<i64>,
    pub conversations: Vec<ConversationItem>,
    pub labels: Vec<LabelItem>,
    pub assignees: Vec<String>,
}

impl From<MRDetails> for MRDetailRes {
    fn from(value: MRDetails) -> Self {
        Self {
            id: value.mr.id,
            link: value.mr.link,
            title: value.mr.title,
            status: value.mr.status.into(),
            open_timestamp: value.mr.created_at.and_utc().timestamp(),
            merge_timestamp: value.mr.merge_date.map(|dt| dt.and_utc().timestamp()),
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

#[derive(Serialize, ToSchema)]
pub struct FilesChangedList {
    pub mui_trees: Vec<MuiTreeNode>,
    pub content: Vec<DiffItem>,
}

#[derive(Serialize, ToSchema)]
pub struct FilesChangedPage {
    pub mui_trees: Vec<MuiTreeNode>,
    pub page: CommonPage<DiffItem>,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct MuiTreeNode {
    id: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(no_recursion)]
    children: Option<Vec<MuiTreeNode>>,
}

impl MuiTreeNode {
    fn new(label: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label: label.to_string(),
            children: None,
        }
    }

    fn insert_path(&mut self, parts: &[&str]) {
        if parts.is_empty() {
            return;
        }

        if self.children.is_none() {
            self.children = Some(Vec::new());
        }

        let children = self.children.as_mut().unwrap();

        if let Some(existing) = children.iter_mut().find(|c| c.label == parts[0]) {
            existing.insert_path(&parts[1..]);
        } else {
            let mut new_node = MuiTreeNode::new(parts[0]);
            new_node.insert_path(&parts[1..]);
            children.push(new_node);
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct MrFilesRes {
    pub path: String,
    pub sha: String,
    pub action: String,
}

impl From<MrDiffFile> for MrFilesRes {
    fn from(value: MrDiffFile) -> Self {
        match value {
            MrDiffFile::New(path, sha) => Self {
                path: path.to_string_lossy().to_string(),
                sha: sha.to_string(),
                action: String::from_str("new").unwrap(),
            },
            MrDiffFile::Deleted(path, sha) => Self {
                path: path.to_string_lossy().to_string(),
                sha: sha.to_string(),
                action: String::from_str("deleted").unwrap(),
            },
            MrDiffFile::Modified(path, _, new) => Self {
                path: path.to_string_lossy().to_string(),
                sha: new.to_string(),
                action: String::from_str("modified").unwrap(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum MergeStatus {
    Open,
    Merged,
    Closed,
}

impl From<MergeStatusEnum> for MergeStatus {
    fn from(value: MergeStatusEnum) -> Self {
        match value {
            MergeStatusEnum::Open => MergeStatus::Open,
            MergeStatusEnum::Merged => MergeStatus::Merged,
            MergeStatusEnum::Closed => MergeStatus::Closed,
        }
    }
}

impl From<MergeStatus> for MergeStatusEnum {
    fn from(value: MergeStatus) -> Self {
        match value {
            MergeStatus::Open => MergeStatusEnum::Open,
            MergeStatus::Merged => MergeStatusEnum::Merged,
            MergeStatus::Closed => MergeStatusEnum::Closed,
        }
    }
}
