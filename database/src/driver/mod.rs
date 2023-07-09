//!
//!
//！

extern crate common;

use std::{collections::HashMap, path::Path};

use async_trait::async_trait;

use common::errors::{GitLFSError, MegaError};
use entity::{commit, node, refs};

use self::lfs::{
    storage::MetaObject,
    structs::{Lock, RequestVars},
};

pub mod lfs;
pub mod mysql;

#[async_trait]
pub trait ObjectStorage: Send + Sync {
    async fn get_head_object_id(&self, path: &Path) -> String;

    async fn get_ref_object_id(&self, path: &Path) -> HashMap<String, String>;

    async fn get_commit_by_hash(&self, hash: &str) -> Result<Option<commit::Model>, MegaError>;

    async fn get_commit_by_id(&self, git_id: String) -> Result<commit::Model, MegaError>;

    async fn get_all_commits_by_path(&self, path: &Path) -> Result<Vec<commit::Model>, MegaError>;

    // get hash object from db if missing cache in unpack process, this object must be tree or blob
    async fn get_hash_object(&self, hash: &str) -> Result<Vec<u8>, MegaError>;

    async fn save_refs(&self, save_models: Vec<refs::ActiveModel>) -> Result<bool, MegaError>;

    async fn update_refs(&self, old_id: String, new_id: String, path: &Path);

    async fn delete_refs(&self, old_id: String, path: &Path);

    async fn get_nodes_by_ids(&self, ids: Vec<String>) -> Result<Vec<node::Model>, MegaError>;

    async fn get_node_by_id(&self, id: &str) -> Option<node::Model>;

    async fn save_nodes(&self, nodes: Vec<node::ActiveModel>) -> Result<bool, MegaError>;

    async fn save_commits(&self, commits: Vec<commit::ActiveModel>) -> Result<bool, MegaError>;

    async fn search_root_node_by_path(&self, repo_path: &Path) -> Option<node::Model>;

    async fn lfs_get_meta(&self, v: &RequestVars) -> Result<MetaObject, GitLFSError>;

    async fn lfs_put_meta(&self, v: &RequestVars) -> Result<MetaObject, GitLFSError>;

    async fn lfs_delete_meta(&self, v: &RequestVars) -> Result<(), GitLFSError>;

    async fn lfs_get_locks(&self, refspec: &str) -> Result<Vec<Lock>, GitLFSError>;

    async fn lfs_get_filtered_locks(
        &self,
        refspec: &str,
        path: &str,
        cursor: &str,
        limit: &str,
    ) -> Result<(Vec<Lock>, String), GitLFSError>;

    async fn lfs_add_lock(&self, refspec: &str, locks: Vec<Lock>) -> Result<(), GitLFSError>;

    async fn lfs_delete_lock(
        &self,
        refspec: &str,
        user: Option<String>,
        id: &str,
        force: bool,
    ) -> Result<Lock, GitLFSError>;
}
