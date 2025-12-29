use std::{path::PathBuf, sync::Arc};

use common::{
    config::{Config, S3Config, StorageType},
    errors::MegaError,
};

use crate::object_storage::{
    ObjectStorage, fs_object_storage::FsObjectStorage, rustfs_object_storage::RustfsObjectStorage,
};

#[derive(Debug, Clone)]
pub enum ObjectStorageConfig {
    Fs { root: PathBuf },
    S3 { config: S3Config },
}

impl ObjectStorageConfig {
    pub fn from_config(storage_type: StorageType, config: Arc<Config>) -> Self {
        match storage_type {
            StorageType::LocalFs => ObjectStorageConfig::Fs {
                root: config.base_dir.clone(),
            },
            StorageType::S3 => ObjectStorageConfig::S3 {
                config: config.s3.clone(),
            },
            StorageType::Database => {
                unimplemented!("Database storage type will be removed in future")
            }
        }
    }
}

pub struct ObjectStorageFactory;

impl ObjectStorageFactory {
    pub async fn create(cfg: ObjectStorageConfig) -> Result<Arc<dyn ObjectStorage>, MegaError> {
        match cfg {
            ObjectStorageConfig::Fs { root } => Ok(Arc::new(FsObjectStorage::new(root))),
            ObjectStorageConfig::S3 { config } => {
                let storage = RustfsObjectStorage::new(config).await?;
                Ok(Arc::new(storage))
            }
        }
    }
}
