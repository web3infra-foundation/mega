pub mod git_storage;
pub mod mega_storage;
pub mod init;

use async_trait::async_trait;
use std::rc::Rc;

use callisto::{git_repo, mega_tree};
use common::errors::MegaError;
use ganymede::mega_node::MegaNode;
use ganymede::model::create_file::CreateFileInfo;
use venus::internal::{
    object::commit::Commit,
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

    async fn get_ref(&self, repo: Repo, refs: RefCommand) -> Result<String, MegaError>;

    async fn update_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError>;

    async fn save_entry(&self, repo: Repo, result_entity: Vec<Entry>) -> Result<(), MegaError>;

    async fn get_entry_by_sha1(
        &self,
        repo: Repo,
        sha1_vec: Vec<&str>,
    ) -> Result<Vec<Entry>, MegaError>;
}

///
/// This interface is designed to handle the specific storage logic of the mega application.
/// It needs to work in conjunction with the data table structure designed for mega.
///
#[async_trait]
pub trait MonorepoStorageProvider: GitStorageProvider + Send {
    async fn init_mega_directory(&self);
    
    fn mega_node_tree(&self, file_infos: Vec<CreateFileInfo>) -> Result<Rc<MegaNode>, MegaError>;

    async fn create_mega_file(&self, file_info: CreateFileInfo) -> Result<(), MegaError>;

    async fn find_git_repo(&self, repo_path: &str) -> Result<Option<git_repo::Model>, MegaError>;

    async fn save_git_repo(&self, repo: Repo) -> Result<(), MegaError>;

    async fn update_git_repo(&self, repo: Repo) -> Result<(), MegaError>;

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

    // async fn save_mega_trees(&self, trees: Vec<Tree>) -> Result<(), MegaError>;
    //
    // async fn save_mega_blobs(&self, blobs: Vec<Blob>) -> Result<(), MegaError>;

    async fn get_mega_tree_by_path(
        &self,
        full_path: &str,
    ) -> Result<Option<mega_tree::Model>, MegaError>;
}
