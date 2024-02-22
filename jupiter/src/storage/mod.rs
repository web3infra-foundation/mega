pub mod git_storage;
pub mod mega_storage;

use async_trait::async_trait;

use common::errors::MegaError;
use venus::internal::{
    object::{commit::Commit, tree::Tree},
    pack::{entry::Entry, reference::RefCommand},
    repo::Repo,
};

#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn save_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError>;

    async fn remove_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError>;

    async fn get_ref(&self, repo: Repo, refs: RefCommand) -> Result<String, MegaError>;

    async fn update_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError>;

    async fn save_entry(&self, repo: Repo, result_entity: Vec<Entry>) -> Result<(), MegaError>;

    async fn get_entry_by_sha1(
        &self,
        repo: Repo,
        sha1_vec: Vec<&str>,
    ) -> Result<Vec<Entry>, MegaError>;
}

#[async_trait]
pub trait DbStorageProvider: StorageProvider {
    async fn save_commits(&self, commits: Vec<Commit>) -> Result<(), MegaError>;

    async fn save_trees(&self, trees: Vec<Tree>) -> Result<(), MegaError>;
}

#[async_trait]
pub trait MegaStorageProvider: StorageProvider {

    async fn save_git_repo(&self) {
        todo!()
    }

    async fn update_git_repo(&self) {
        todo!()
    }

    async fn save_git_trees(&self) {
        todo!()
    }

    async fn save_git_commits(
        &self,
        repo_id: i64,
        full_path: &str,
        commits: Vec<Commit>,
    ) -> Result<(), MegaError>;

    async fn save_mega_commits(
        &self,
        mr_id: &str,
        full_path: &str,
        commits: Vec<Commit>,
    ) -> Result<(), MegaError>;
}
