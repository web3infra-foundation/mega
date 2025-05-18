use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use mercury::internal::object::{
    commit::Commit,
    tree::{TreeItem, TreeItemMode},
};

#[derive(PartialEq, Eq, Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateFileInfo {
    /// can be a file or directory
    pub is_directory: bool,
    pub name: String,
    /// leave empty if it's under root
    pub path: String,
    // pub import_dir: bool,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct CodePreviewQuery {
    #[serde(default)]
    pub refs: String,
    #[serde(default = "default_path")]
    pub path: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct TreeQuery {
    pub oid: Option<String>,
    #[serde(default = "default_path")]
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct BlobContentQuery {
    #[serde(default = "default_path")]
    pub path: String,
}

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
pub struct TreeCommitItem {
    pub oid: String,
    pub name: String,
    pub content_type: String,
    pub message: String,
    pub date: String,
}

impl From<(TreeItem, Option<Commit>)> for TreeCommitItem {
    fn from((item, commit): (TreeItem, Option<Commit>)) -> Self {
        TreeCommitItem {
            name: item.name.clone(),
            content_type: if item.mode == TreeItemMode::Tree {
                "directory".to_owned()
            } else {
                "file".to_owned()
            },
            oid: commit
                .as_ref()
                .map(|x| x.id.to_string())
                .unwrap_or_default(),
            message: commit
                .as_ref()
                .map(|x| x.format_message())
                .unwrap_or_default(),
            date: commit
                .as_ref()
                .map(|x| x.committer.timestamp.to_string())
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
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

fn default_path() -> String {
    "/".to_string()
}
