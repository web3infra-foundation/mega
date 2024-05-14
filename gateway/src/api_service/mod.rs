use std::path::PathBuf;

use axum::async_trait;

use mercury::{errors::GitError, internal::object::commit::Commit};

use crate::model::objects::{
    BlobObjects, LatestCommitInfo, TreeBriefInfo, TreeCommitInfo, UserInfo,
};

pub mod import_service;
pub mod mono_service;
pub mod router;

const SIGNATURE_END: &str = "-----END PGP SIGNATURE-----";

#[async_trait]
pub trait ApiHandler: Send + Sync {
    async fn get_blob_as_string(&self, object_id: &str) -> Result<BlobObjects, GitError>;

    async fn get_latest_commit(&self, path: PathBuf) -> Result<LatestCommitInfo, GitError>;

    async fn get_tree_info(&self, path: PathBuf) -> Result<TreeBriefInfo, GitError>;

    async fn get_tree_commit_info(&self, path: PathBuf) -> Result<TreeCommitInfo, GitError>;

    fn convert_commit_to_info(&self, commit: Commit) -> Result<LatestCommitInfo, GitError> {
        let message = self.remove_useless_str(commit.message.clone(), SIGNATURE_END.to_owned());
        let committer = UserInfo {
            display_name: commit.committer.name,
            ..Default::default()
        };
        let author = UserInfo {
            display_name: commit.author.name,
            ..Default::default()
        };

        let res = LatestCommitInfo {
            oid: commit.id.to_plain_str(),
            date: commit.committer.timestamp.to_string(),
            short_message: message,
            author,
            committer,
            status: "success".to_string(),
        };
        Ok(res)
    }

    fn remove_useless_str(&self, content: String, remove_str: String) -> String {
        if let Some(index) = content.find(&remove_str) {
            let filtered_text = &content[index + remove_str.len()..].replace('\n', "");
            let truncated_text = filtered_text.chars().take(50).collect::<String>();
            truncated_text.to_owned()
        } else {
            "".to_owned()
        }
    }
}
