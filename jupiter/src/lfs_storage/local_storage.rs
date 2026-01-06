use std::fs::{self};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;

use common::config::LFSLocalConfig;
use common::errors::MegaError;
use sea_orm::DatabaseConnection;

use crate::lfs_storage::{LfsFileStorage, transform_path};
use crate::storage::base_storage::{BaseStorage, StorageConnector};
use crate::storage::lfs_db_storage::LfsDbStorage;

pub struct LocalStorage {
    config: LFSLocalConfig,
    #[allow(dead_code)]
    lfs_db_storage: LfsDbStorage,
}

impl LocalStorage {
    pub fn init(config: LFSLocalConfig, connection: Arc<DatabaseConnection>) -> LocalStorage {
        fs::create_dir_all(&config.lfs_file_path).expect("Create directory failed!");
        LocalStorage {
            config,
            lfs_db_storage: LfsDbStorage {
                base: BaseStorage::new(connection),
            },
        }
    }

    pub fn mock() -> Self {
        Self {
            config: LFSLocalConfig::default(),
            lfs_db_storage: LfsDbStorage {
                base: BaseStorage::mock(),
            },
        }
    }
}

#[async_trait]
impl LfsFileStorage for LocalStorage {
    async fn get_object(&self, object_id: &str) -> Result<Bytes, MegaError> {
        let path = Path::new(&self.config.lfs_file_path)
            .join("objects")
            .join(transform_path(object_id));
        let mut file =
            fs::File::open(&path).unwrap_or_else(|_| panic!("Open file:{path:?} failed!"));
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        Ok(Bytes::from(buffer))
    }

    async fn download_url(&self, object_id: &str, hostname: &str) -> Result<String, MegaError> {
        Ok(self.action_href(object_id, hostname))
    }

    async fn put_object(&self, object_id: &str, body_content: Vec<u8>) -> Result<(), MegaError> {
        let path = Path::new(&self.config.lfs_file_path)
            .join("objects")
            .join(transform_path(object_id));
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir).expect("Create directory failed!");

        let mut file = fs::File::create(&path).expect("Open file failed");
        file.write_all(&body_content).expect("Write file failed");
        Ok(())
    }

    async fn exist_object(&self, object_id: &str) -> bool {
        exist_object(self.config.lfs_file_path.clone(), object_id)
    }
}

fn exist_object(path: PathBuf, object_id: &str) -> bool {
    let path = Path::new(&path)
        .join("objects")
        .join(transform_path(object_id));
    Path::exists(&path)
}

#[cfg(test)]
mod tests {
    use crate::lfs_storage::{LfsFileStorage, local_storage::LocalStorage};

    #[tokio::test]
    async fn test_content_store() {
        let oid = "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72".to_owned();
        let content = "test content".as_bytes().to_vec();

        let local_storage = LocalStorage::mock();
        assert!(local_storage.put_object(&oid, content).await.is_ok());
        assert!(local_storage.exist_object(&oid).await);
    }
}
