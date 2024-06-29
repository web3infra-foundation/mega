use callisto::mega_mr;
use serde::{Deserialize, Serialize};
use mercury::internal::object::tree::{TreeItem, TreeItemMode};

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

#[derive(Serialize, Deserialize)]
pub struct MrInfoItem {
    pub title: String,
    pub status: String,
    pub open_date: String,
    pub merge_date: String,
}

impl From<mega_mr::Model> for MrInfoItem {
    fn from(value: mega_mr::Model) -> Self {
        Self {
            title: String::new(),
            status: value.status.to_string(),
            open_date: value.created_at.to_string(),
            merge_date: value.merge_date.unwrap().to_string(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Default, Serialize, Deserialize)]

pub struct CommonResult<T> {
    pub result: bool,
    pub data: Option<T>,
    pub err_message: String,
}

impl <T> CommonResult<T> {
    pub fn success(data: Option<T>) -> Self {
        CommonResult {
            result: true,
            data,
            err_message: "".to_owned(),
        }
    }
    pub fn failed(err_message: &str) -> Self {
        CommonResult {
            result: false,
            data: None,
            err_message: err_message.to_string(),
        }
    }
}
