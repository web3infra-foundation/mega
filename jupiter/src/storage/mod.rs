pub mod git_storage;
pub mod mega_storage;

use async_trait::async_trait;
use std::rc::Rc;

use common::errors::MegaError;
use db_entity::git_repo;
use venus::internal::{
    object::{commit::Commit, tree::Tree},
    pack::{entry::Entry, reference::RefCommand},
    repo::Repo,
};
use venus::model::create_file::CreateFileInfo;
use venus::model::mega_node::MegaNode;

///
/// This interface is designed to handle the commonalities between the git storage
/// and the mega application storage.
///
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

///
/// This interface is designed to handle the specific storage logic of the mega application.
/// It needs to work in conjunction with the data table structure designed for mega.
///
#[async_trait]
pub trait MegaStorageProvider: StorageProvider + Send {
    fn mega_node_tree(
        &self,
        file_infos: Vec<CreateFileInfo>,
    ) -> Result<Rc<MegaNode>, MegaError>;

    async fn search_snapshot(&self) {}

    async fn find_git_repo(&self, repo_path: &str) -> Result<Option<git_repo::Model>, MegaError>;

    async fn save_git_repo(&self, repo: Repo) -> Result<(), MegaError>;

    async fn update_git_repo(&self, repo: Repo) -> Result<(), MegaError>;

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
