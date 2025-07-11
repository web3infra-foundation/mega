use std::str::FromStr;

use ceres::model::mr::MrDiffFile;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use callisto::{
    mega_conversation, mega_mr,
    sea_orm_active_enums::{ConvTypeEnum, MergeStatusEnum},
};

pub mod mr_router;

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
    pub username: String,
    pub conv_type: ConvTypeEnum,
    pub comment: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<mega_conversation::Model> for MegaConversation {
    fn from(value: mega_conversation::Model) -> Self {
        Self {
            id: value.id,
            username: value.username,
            conv_type: value.conv_type,
            comment: value.comment,
            created_at: value.created_at.and_utc().timestamp(),
            updated_at: value.updated_at.and_utc().timestamp(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct FilesChangedList {
    pub mui_trees: Vec<MuiTreeNode>,
    pub content: String,
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
