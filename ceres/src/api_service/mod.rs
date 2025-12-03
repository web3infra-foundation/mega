use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;

use callisto::raw_blob;
use common::config::BlameConfig;
use common::errors::MegaError;
use common::model::{DiffItem, Pagination};
use git_internal::{
    errors::GitError,
    internal::object::{
        commit::Commit,
        tree::{Tree, TreeItem},
    },
};
use jupiter::storage::Storage;

use crate::model::{
    blame::{BlameQuery, BlameResult},
    tag::TagInfo,
};
use crate::{
    api_service::cache::GitObjectCache,
    model::git::{
        CommitBindingInfo, CreateEntryInfo, DiffPreviewPayload, EditFilePayload, EditFileResult,
        LatestCommitInfo, TreeBriefItem, TreeCommitItem, TreeHashItem,
    },
};

pub mod blame_ops;
pub mod blob_ops;
pub mod cache;
pub mod commit_ops;
pub mod history;
pub mod import_api_service;
pub mod mono_api_service;
pub mod state;
pub mod tree_ops;

#[async_trait]
pub trait ApiHandler: Send + Sync {
    fn get_context(&self) -> Storage;

    fn object_cache(&self) -> &GitObjectCache;

    fn strip_relative(&self, path: &Path) -> Result<PathBuf, MegaError>;

    async fn get_root_tree(&self, refs: Option<&str>) -> Result<Tree, MegaError>;

    async fn get_tree_by_hash(&self, hash: &str) -> Result<Tree, MegaError>;

    async fn get_tree_info(
        &self,
        path: &Path,
        refs: Option<&str>,
    ) -> Result<Vec<TreeBriefItem>, GitError> {
        tree_ops::get_tree_info(self, path, refs).await
    }
    async fn get_binary_tree_by_path(
        &self,
        path: &Path,
        oid: Option<String>,
    ) -> Result<Vec<u8>, MegaError> {
        tree_ops::get_binary_tree_by_path(self, path, oid).await
    }

    async fn get_tree_commit_info(
        &self,
        path: PathBuf,
        refs: Option<&str>,
    ) -> Result<Vec<TreeCommitItem>, GitError> {
        tree_ops::get_tree_commit_info(self, path, refs).await
    }

    async fn item_to_commit_map(
        &self,
        path: PathBuf,
        reference: Option<&str>,
    ) -> Result<HashMap<TreeItem, Option<Commit>>, GitError>;

    /// return the dir's hash only
    async fn get_tree_dir_hash(
        &self,
        path: PathBuf,
        dir_name: &str,
        refs: Option<&str>,
    ) -> Result<Vec<TreeHashItem>, GitError> {
        tree_ops::get_tree_dir_hash(self, path, dir_name, refs).await
    }

    async fn get_commit_by_hash(&self, hash: &str) -> Result<Commit, MegaError>;

    /// Get the latest commit that modified a file or directory.
    /// Unified method that handles both tag-based and commit-based browsing.
    async fn get_latest_commit(
        &self,
        path: PathBuf,
        refs: Option<&str>,
    ) -> Result<LatestCommitInfo, GitError> {
        commit_ops::get_latest_commit(self, path, refs).await
    }

    async fn get_commits_by_hashes(&self, c_hashes: Vec<String>) -> Result<Vec<Commit>, GitError>;

    async fn get_blob_as_string(
        &self,
        file_path: PathBuf,
        refs: Option<&str>,
    ) -> Result<Option<String>, GitError> {
        blob_ops::get_blob_as_string(self, file_path, refs).await
    }

    /// Create a file or directory entry under the monorepo path. Returns the new commit id on success.
    async fn create_monorepo_entry(&self, file_info: CreateEntryInfo) -> Result<String, GitError>;

    async fn get_raw_blob_by_hash(&self, hash: &str) -> Result<Option<raw_blob::Model>, MegaError> {
        let context = self.get_context();
        context.raw_db_storage().get_raw_blob_by_hash(hash).await
    }

    /// Preview unified diff for a single file change
    async fn preview_file_diff(
        &self,
        payload: DiffPreviewPayload,
    ) -> Result<Option<DiffItem>, GitError> {
        blob_ops::preview_file_diff(self, payload).await
    }

    /// Build commit binding information for a given commit SHA
    async fn build_commit_binding_info(
        &self,
        commit_sha: &str,
    ) -> Result<Option<CommitBindingInfo>, GitError> {
        commit_ops::build_commit_binding_info(self, commit_sha).await
    }

    async fn get_root_commit(&self) -> Result<Commit, MegaError>;

    /// List commit history with optional refs, path filter, author filter, and pagination.
    /// total is the total number of commits matching the criteria
    async fn list_commit_history(
        &self,
        refs: Option<&str>,
        path_filter: Option<&std::path::PathBuf>,
        author: Option<&str>,
        page: Pagination,
    ) -> Result<(Vec<crate::model::commit::CommitSummary>, u64), GitError> {
        commit_ops::list_commit_history(self, refs, path_filter, author, page).await
    }

    /// Build commit detail including summary and diffs for a given commit SHA.
    /// The `path` acts as a required repository/subrepo selector for routing/cache,
    /// and does not filter diff contents.
    async fn build_commit_detail(
        &self,
        commit_sha: &str,
        path: &std::path::Path,
    ) -> Result<crate::model::commit::CommitDetail, GitError> {
        commit_ops::build_commit_detail(self, commit_sha, path).await
    }

    // Tag related operations shared across mono/import implementations.
    /// Create a tag in the repository context represented by `repo_path`.
    /// Returns TagInfo on success.
    async fn create_tag(
        &self,
        repo_path: Option<String>,
        name: String,
        target: Option<String>,
        tagger_name: Option<String>,
        tagger_email: Option<String>,
        message: Option<String>,
    ) -> Result<TagInfo, GitError>;

    /// List tags under the repository context represented by `repo_path`.
    /// Returns (items, total_count) according to Pagination.
    async fn list_tags(
        &self,
        repo_path: Option<String>,
        pagination: Pagination,
    ) -> Result<(Vec<TagInfo>, u64), GitError>;

    /// Get a tag by name under the repository context represented by `repo_path`.
    async fn get_tag(
        &self,
        repo_path: Option<String>,
        name: String,
    ) -> Result<Option<TagInfo>, GitError>;

    /// Delete a tag by name under the repository context represented by `repo_path`.
    async fn delete_tag(&self, repo_path: Option<String>, name: String) -> Result<(), GitError>;

    /// Get blame configuration from storage config.
    /// Returns default config if storage config is not available.
    fn get_blame_config(&self) -> BlameConfig {
        self.get_context()
            .config
            .upgrade()
            .map(|c| c.blame.clone())
            .unwrap_or_default()
    }

    /// Get blame information for a file
    async fn get_file_blame(
        &self,
        file_path: &str,
        ref_name: Option<&str>,
        query: BlameQuery,
    ) -> Result<BlameResult, GitError> {
        blame_ops::get_file_blame(self, file_path, ref_name, query).await
    }

    /// Save file edit with conflict detection and commit creation.
    async fn save_file_edit(&self, payload: EditFilePayload) -> Result<EditFileResult, GitError>;

    /// the dir's hash as same as old,file's hash is the content hash
    /// may think about change dir'hash as the content
    /// for now,only change the file's hash
    async fn get_tree_content_hash(
        &self,
        path: PathBuf,
        refs: Option<&str>,
    ) -> Result<Vec<TreeHashItem>, GitError> {
        tree_ops::get_tree_content_hash(self, path, refs).await
    }

    /// Searches for a tree in the Git repository by its path and returns the trees involved in the update and the target tree.
    async fn search_tree_for_update(&self, path: &Path) -> Result<Vec<Arc<Tree>>, GitError> {
        tree_ops::search_tree_for_update(self, path).await
    }
}
