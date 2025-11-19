use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use callisto::{check_result, sea_orm_active_enums::MergeStatusEnum};
use common::model::CommonPage;
use common::model::DiffItem;
use git_internal::hash::SHA1;
use jupiter::model::cl_dto::CLDetails;
use jupiter::model::common::ListParams;

use crate::merge_checker::{CheckType, ConditionResult};
use crate::model::{conversation::ConversationItem, label::LabelItem};

#[derive(Deserialize, ToSchema)]
pub struct AssigneeUpdatePayload {
    pub assignees: Vec<String>,
    pub item_id: i64,
    pub link: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ListPayload {
    pub status: String,
    pub author: Option<String>,
    pub labels: Option<Vec<i64>>,
    pub assignees: Option<Vec<String>>,
    pub sort_by: Option<String>,
    pub asc: bool,
}

impl From<ListPayload> for ListParams {
    fn from(value: ListPayload) -> Self {
        Self {
            status: value.status,
            author: value.author,
            labels: value.labels,
            assignees: value.assignees,
            sort_by: value.sort_by,
            asc: value.asc,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct CLDetailRes {
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

impl From<CLDetails> for CLDetailRes {
    fn from(value: CLDetails) -> Self {
        Self {
            id: value.cl.id,
            link: value.cl.link,
            title: value.cl.title,
            status: value.cl.status.into(),
            open_timestamp: value.cl.created_at.and_utc().timestamp(),
            merge_timestamp: value.cl.merge_date.map(|dt| dt.and_utc().timestamp()),
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
pub struct FilesChangedPage {
    pub page: CommonPage<DiffItem>,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct MuiTreeNode {
    id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(no_recursion)]
    pub children: Option<Vec<MuiTreeNode>>,
}

impl MuiTreeNode {
    pub fn new(label: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label: label.to_string(),
            children: None,
        }
    }

    pub fn insert_path(&mut self, parts: &[&str]) {
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
pub struct ClFilesRes {
    pub path: String,
    pub sha: String,
    pub action: String,
}

impl From<ClDiffFile> for ClFilesRes {
    fn from(value: ClDiffFile) -> Self {
        match value {
            ClDiffFile::New(path, sha) => Self {
                path: path.to_string_lossy().to_string(),
                sha: sha.to_string(),
                action: String::from_str("new").unwrap(),
            },
            ClDiffFile::Deleted(path, sha) => Self {
                path: path.to_string_lossy().to_string(),
                sha: sha.to_string(),
                action: String::from_str("deleted").unwrap(),
            },
            ClDiffFile::Modified(path, _, new) => Self {
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
            result: value.status.parse().unwrap(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub enum RequirementsState {
    UNMERGEABLE,
    MERGEABLE,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[allow(dead_code)]
pub struct VerifyClPayload {
    pub assignees: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ReviewerPayload {
    pub reviewer_usernames: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ReviewersResponse {
    pub result: Vec<ReviewerInfo>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ChangeReviewerStatePayload {
    pub approved: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ChangeReviewStatePayload {
    pub conversation_id: i64,
    pub resolved: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ReviewerInfo {
    pub username: String,
    pub approved: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct CloneRepoPayload {
    pub owner: String,
    pub repo: String,
    pub path: String,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum ClDiffFile {
    New(PathBuf, SHA1),
    Deleted(PathBuf, SHA1),
    // path, old_hash, new_hash
    Modified(PathBuf, SHA1, SHA1),
}

impl ClDiffFile {
    pub fn path(&self) -> &PathBuf {
        match self {
            ClDiffFile::New(path, _) => path,
            ClDiffFile::Deleted(path, _) => path,
            ClDiffFile::Modified(path, _, _) => path,
        }
    }

    pub fn kind_weight(&self) -> u8 {
        match self {
            ClDiffFile::New(_, _) => 0,
            ClDiffFile::Deleted(_, _) => 1,
            ClDiffFile::Modified(_, _, _) => 2,
        }
    }
}

#[derive(Serialize)]
pub struct BuckFile {
    pub buck: SHA1,
    pub buck_config: SHA1,
    pub path: PathBuf,
}
