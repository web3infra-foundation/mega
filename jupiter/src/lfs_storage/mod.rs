use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use aws_s3_storage::AwsS3Storage;
use bytes::Bytes;

use callisto::sea_orm_active_enums::StorageTypeEnum;
use common::{
    config::LFSConfig,
    errors::{GitLFSError, MegaError},
};

use crate::{lfs_storage::local_storage::LocalStorage, storage::lfs_db_storage::LfsDbStorage};

mod aws_s3_storage;
pub mod local_storage;

#[async_trait]
pub trait LfsFileStorage: Sync + Send {
    async fn get_object(&self, object_id: &str) -> Result<Bytes, MegaError>;

    fn action_href(&self, object_id: &str, hostname: &str) -> String {
        let path = PathBuf::new()
            .join(hostname)
            .join("objects")
            .join(object_id);
        path.into_os_string().into_string().unwrap()
    }

    async fn download_url(&self, object_id: &str, hostname: &str) -> Result<String, MegaError>;

    async fn upload_url(&self, object_id: &str, hostname: &str) -> Result<String, MegaError> {
        Ok(self.action_href(object_id, hostname))
    }
    async fn put_object(&self, object_id: &str, body_content: Vec<u8>) -> Result<(), MegaError>;

    async fn put_object_with_chunk(
        &self,
        object_id: &str,
        body_content: &[u8],
        split_size: usize,
    ) -> Result<(), GitLFSError>;

    async fn exist_object(&self, object_id: &str, enable_split: bool) -> bool;
}

fn transform_path(sha1: &str) -> String {
    if sha1.len() < 5 {
        sha1.to_string()
    } else {
        Path::new(&sha1[0..2])
            .join(&sha1[2..4])
            .join(&sha1[4..sha1.len()])
            .into_os_string()
            .into_string()
            .unwrap()
    }
}

pub async fn init(lfs_config: LFSConfig, lfs_db_storage: LfsDbStorage) -> Arc<dyn LfsFileStorage> {
    match lfs_config.storage_type {
        StorageTypeEnum::LocalFs => Arc::new(LocalStorage::init(lfs_config.local, lfs_db_storage)),
        StorageTypeEnum::AwsS3 => Arc::new(AwsS3Storage::init(lfs_config.aws).await),
        _ => unreachable!("Not supported value of config `storage_type`, support value can be 'local_fs' or 'aws_s3'"),
    }
}
