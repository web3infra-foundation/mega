pub mod git_storage;
pub mod init;
pub mod mega_storage;

use async_trait::async_trait;

use common::errors::MegaError;
use venus::internal::{
    pack::{entry::Entry, reference::RefCommand},
    repo::Repo,
};

///
/// This interface is designed to handle the commonalities between the git storage
/// and the mega monerepo storage.
///
#[async_trait]
pub trait GitStorageProvider: Send + Sync {
    async fn save_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError>;

    async fn remove_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError>;

    async fn get_ref(&self, repo: &Repo, ref_name: &str) -> Result<String, MegaError>;

    async fn update_ref(&self, repo: &Repo, ref_name: &str, new_id: &str) -> Result<(), MegaError>;

    async fn save_entry(&self, repo: Repo, result_entity: Vec<Entry>) -> Result<(), MegaError>;

    async fn get_entry_by_sha1(
        &self,
        repo: Repo,
        sha1_vec: Vec<&str>,
    ) -> Result<Vec<Entry>, MegaError>;
}
