//!
//!
//！

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use async_trait::async_trait;
use megacore::errors::{GitLFSError, MegaError};

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

    async fn get_full_pack_data(&self, repo_path: &Path) -> Result<Vec<u8>, MegaError>;

    async fn get_incremental_pack_data(
        &self,
        repo_path: &Path,
        want: &HashSet<String>,
        have: &HashSet<String>,
    ) -> Result<Vec<u8>, MegaError>;

    async fn get_commit_by_hash(&self, hash: &str) -> Result<Vec<u8>, MegaError>;

    // get hash object from db if missing cache in unpack process, this object must be tree or blob
    async fn get_hash_object(&self, hash: &str) -> Result<Vec<u8>, MegaError>;

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
