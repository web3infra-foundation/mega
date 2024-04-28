use std::{
    env,
    path::{self, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use bytes::Bytes;

use common::errors::MegaError;

use crate::driver::file_storage::local_storage::LocalStorage;
use crate::driver::file_storage::remote_storage::RemoteStorage;

pub mod local_storage;
pub mod remote_storage;
pub mod s3_service;

#[async_trait]
pub trait FileStorage: Sync + Send {
    async fn get(&self, object_id: &str) -> Result<Bytes, MegaError>;

    async fn put(
        &self,
        object_id: &str,
        size: i64,
        body_content: &[u8],
    ) -> Result<String, MegaError>;

    fn exist(&self, object_id: &str) -> bool;

    async fn list(&self) {
        unreachable!("not implement")
    }

    async fn delete(&self) {
        unreachable!("not implement")
    }

    fn transform_path(&self, path: &str) -> String {
        if path.len() < 5 {
            path.to_string()
        } else {
            path::Path::new(&path[0..2])
                .join(&path[2..4])
                .join(&path[4..path.len()])
                .into_os_string()
                .into_string()
                .unwrap()
        }
    }
}

pub async fn init(path: String) -> Arc<dyn FileStorage> {
    let storage_type = env::var("MEGA_OBJ_STORAGR_TYPE").unwrap();
    match storage_type.as_str() {
        "LOCAL" => {
            let mut base_path = PathBuf::from(env::var("MEGA_OBJ_LOCAL_PATH").unwrap());
            base_path.push(path);
            Arc::new(LocalStorage::init(base_path))
        }
        "REMOTE" => Arc::new(RemoteStorage::init(path).await),
        _ => unreachable!(
            "Not supported config, MEGA_OBJ_STORAGR_TYPE should be 'LOCAL' or 'REMOTE'"
        ),
    }
}
