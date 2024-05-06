use axum::async_trait;

use venus::errors::GitError;

use crate::model::objects::{LatestCommitInfo, TreeCommitInfo};

pub mod mono_service;
pub mod obj_service;
pub mod router;

#[async_trait]
pub trait ApiHandler: Send + Sync {
    async fn get_latest_commit(&self) -> Result<LatestCommitInfo, GitError>;

    #[allow(dead_code)] // @benjamin.747
    async fn get_tree_commit_info(&self) -> Result<TreeCommitInfo, GitError>;
}
