use serde::{Deserialize, Serialize};
use venus::internal::object::tree::{TreeItem, TreeItemMode};

#[derive(Serialize, Deserialize)]
pub struct LatestCommitInfo {
    pub oid: String,
    pub date: String,
    pub short_message: String,
    pub author: UserInfo,
    pub committer: UserInfo,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    pub display_name: String,
    pub avatar_url: String,
}

impl Default for UserInfo {
    fn default() -> Self {
        UserInfo {
            display_name: String::default(),
            avatar_url: "default_url".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TreeCommitInfo {
    pub items: Vec<TreeCommitItem>,
    pub total_count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct TreeCommitItem {
    pub oid: String,
    pub name: String,
    pub content_type: String,
    pub message: String,
    pub date: String,
}

impl From<TreeItem> for TreeCommitItem {
    fn from(value: TreeItem) -> Self {
        TreeCommitItem {
            name: value.name,
            content_type: if value.mode == TreeItemMode::Tree {
                "directory".to_owned()
            } else {
                "file".to_owned()
            },
            oid: String::new(),
            message: String::new(),
            date: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TreeBriefInfo {
    pub items: Vec<TreeBriefItem>,
    pub total_count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct TreeBriefItem {
    pub name: String,
    pub path: String,
    pub content_type: String,
}

impl From<TreeItem> for TreeBriefItem {
    fn from(value: TreeItem) -> Self {
        TreeBriefItem {
            name: value.name,
            path: String::new(),
            content_type: if value.mode == TreeItemMode::Tree {
                "directory".to_owned()
            } else {
                "file".to_owned()
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BlobObjects {
    pub plain_text: String,
}
