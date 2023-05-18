//!
//!
//！

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use async_trait::async_trait;
use mega_core::errors::MegaError;

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
}
