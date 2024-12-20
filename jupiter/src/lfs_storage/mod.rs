use std::{
    path::{self, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use bytes::Bytes;

use common::errors::MegaError;

use crate::lfs_storage::local_storage::LocalStorage;

pub mod local_storage;

#[async_trait]
pub trait LfsStorage: Sync + Send {
    async fn get_ref(&self, repo_id: i64, ref_name: &str) -> Result<String, MegaError>;

    async fn put_ref(&self, repo_id: i64, ref_name: &str, ref_hash: &str) -> Result<(), MegaError>;

    async fn delete_ref(&self, repo_id: i64, ref_name: &str) -> Result<(), MegaError>;

    async fn update_ref(
        &self,
        repo_id: i64,
        ref_name: &str,
        ref_hash: &str,
    ) -> Result<(), MegaError>;

    async fn get_object(&self, object_id: &str) -> Result<Bytes, MegaError>;

    async fn put_object(&self, object_id: &str, body_content: &[u8]) -> Result<String, MegaError>;

    fn exist_object(&self, object_id: &str) -> bool;

    fn transform_path(&self, sha1: &str) -> String {
        if sha1.len() < 5 {
            sha1.to_string()
        } else {
            path::Path::new(&sha1[0..2])
                .join(&sha1[2..4])
                .join(&sha1[4..sha1.len()])
                .into_os_string()
                .into_string()
                .unwrap()
        }
    }
}

pub async fn init(storage_type: String, base_path: PathBuf) -> Arc<dyn LfsStorage> {
    match storage_type.as_str() {
        "LOCAL" => Arc::new(LocalStorage::init(base_path)),
        // "REMOTE" => Arc::new(RemoteStorage::init(path).await),
        _ => unreachable!(
            "Not supported config, MEGA_OBJ_STORAGE_TYPE should be 'LOCAL' or 'REMOTE'"
        ),
    }
}

pub fn mock() -> Arc<dyn LfsStorage> {
    Arc::new(LocalStorage::init(PathBuf::from("/")))
}
