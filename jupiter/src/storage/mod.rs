pub mod git_storage;
pub mod mega_storage;

use async_trait::async_trait;

use common::errors::MegaError;
use venus::internal::{
    object::commit::Commit,
    pack::{entry::Entry, reference::RefCommand},
};

#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn save_ref(&self, refs: RefCommand) -> Result<(), MegaError>;

    async fn remove_ref(&self, refs: RefCommand) -> Result<(), MegaError>;

    async fn get_ref(&self, refs: RefCommand) -> Result<String, MegaError>;

    async fn update_ref(&self, refs: RefCommand) -> Result<(), MegaError>;

    async fn save_entry(&self, result_entity: Vec<Entry>) -> Result<(), MegaError>;

    async fn get_entry_by_sha1(&self, sha1_vec: Vec<&str>) -> Result<Vec<Entry>, MegaError>;
}

#[async_trait]
pub trait DbStorageProvider: StorageProvider {
    async fn save_commits(&self, commits: Vec<Commit>) -> Result<(), MegaError>;
}

#[async_trait]
pub trait MegaStorageProvider: StorageProvider {
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
