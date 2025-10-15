use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use git_internal::internal::object::{
    commit::Commit,
    tree::{TreeItem, TreeItemMode},
};

#[derive(PartialEq, Eq, Debug, Clone, Deserialize, ToSchema)]
pub struct CreateEntryInfo {
    /// can be a file or directory
    pub is_directory: bool,
    pub name: String,
    /// leave empty if it's under root
    pub path: String,
    // pub import_dir: bool,
    pub content: Option<String>,
}

impl CreateEntryInfo {
    pub fn commit_msg(&self) -> String {
        if self.is_directory {
            format!("\n create new directory {}", self.name)
        } else {
            format!("\n create new file {}", self.name)
        }
    }
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

#[derive(Debug, Deserialize, IntoParams)]
pub struct BlobContentQuery {
    #[serde(default = "default_path")]
    pub path: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LatestCommitInfo {
    pub oid: String,
    pub date: String,
    pub short_message: String,
    pub author: UserInfo,
    pub committer: UserInfo,
    pub status: String,
    pub binding_info: Option<CommitBindingInfo>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CommitBindingInfo {
    pub matched_username: Option<String>,
    pub is_anonymous: bool,
    pub is_verified_user: bool,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub author_email: String,
}

impl From<Commit> for LatestCommitInfo {
    fn from(commit: Commit) -> Self {
        let message = commit.format_message();
        let committer = UserInfo {
            display_name: commit.committer.name,
            ..Default::default()
        };
        let author = UserInfo {
            display_name: commit.author.name,
            ..Default::default()
        };
        Self {
            oid: commit.id.to_string(),
            date: commit.committer.timestamp.to_string(),
            short_message: message,
            author,
            committer,
            status: "success".to_string(),
            binding_info: None, // Will be populated at API layer
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
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

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TreeCommitItem {
    pub commit_id: String,
    pub name: String,
    pub content_type: String,
    pub commit_message: String,
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
            commit_id: commit
                .as_ref()
                .map(|x| x.id.to_string())
                .unwrap_or_default(),
            commit_message: commit
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

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TreeHashItem {
    pub name: String,
    pub content_type: String,
    pub oid: String,
}

impl From<TreeItem> for TreeHashItem {
    fn from(value: TreeItem) -> Self {
        Self {
            oid: value.id.to_string(),
            name: value.name,
            content_type: if value.mode == TreeItemMode::Tree {
                "directory".to_owned()
            } else {
                "file".to_owned()
            },
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TreeResponse {
    pub file_tree: HashMap<String, FileTreeItem>,
    pub tree_items: Vec<TreeBriefItem>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FileTreeItem {
    pub tree_items: Vec<TreeBriefItem>,
    pub total_count: usize,
}
