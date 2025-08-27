use std::str::FromStr;

use ceres::{merge_checker::CheckType, model::mr::MrDiffFile};
use jupiter::model::mr_dto::MRDetails;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::{conversation::ConversationItem, label::LabelItem};
use callisto::{check_result, sea_orm_active_enums::MergeStatusEnum};
use common::model::CommonPage;
use neptune::model::diff_model::DiffItem;

mod model;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
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

#[derive(Serialize, ToSchema)]
pub struct MergeBoxRes {
    pub merge_requirements: Option<MergeRequirements>,
}

impl MergeBoxRes {
    pub fn from_condition(conditions: Vec<Condition>) -> Self {
        let mut state = RequirementsState::MERGEABLE;
        for cond in &conditions {
            if cond.result == ConditionResult::FAILED {
                state = RequirementsState::UNMERGEABLE
            }
        }
        MergeBoxRes {
            merge_requirements: Some(MergeRequirements { conditions, state }),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct MergeRequirements {
    pub conditions: Vec<Condition>,
    pub state: RequirementsState,
}

#[derive(Serialize, ToSchema)]
pub struct Condition {
    #[serde(rename = "type")]
    pub condition_type: CheckType,
    pub display_name: String,
    pub description: String,
    pub message: String,
    pub result: ConditionResult,
}

impl From<check_result::Model> for Condition {
    fn from(value: check_result::Model) -> Self {
        let check_type: CheckType = value.check_type_code.into();
        Self {
            condition_type: check_type.clone(),
            display_name: check_type.clone().display_name().to_string(),
            description: check_type.description().to_string(),
            message: value.message,
            result: ConditionResult::PASSED,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub enum RequirementsState {
    UNMERGEABLE,
    MERGEABLE,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub enum ConditionResult {
    FAILED,
    PASSED,
}
