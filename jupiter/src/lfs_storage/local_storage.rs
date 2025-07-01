use std::cmp::min;
use std::fs::{self};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;

use callisto::lfs_split_relations;
use common::config::LFSLocalConfig;
use common::errors::{GitLFSError, MegaError};

use crate::lfs_storage::{transform_path, LfsFileStorage};
use crate::storage::lfs_db_storage::LfsDbStorage;

pub struct LocalStorage {
    config: LFSLocalConfig,
    lfs_db_storage: LfsDbStorage,
}

impl LocalStorage {
    pub fn init(config: LFSLocalConfig, lfs_db_storage: LfsDbStorage) -> LocalStorage {
        fs::create_dir_all(&config.lfs_file_path).expect("Create directory failed!");
        LocalStorage {
            config,
            lfs_db_storage,
        }
    }

    pub fn mock() -> Self {
        Self {
            config: LFSLocalConfig::default(),
            lfs_db_storage: LfsDbStorage::mock(),
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

    async fn put_object_with_chunk(
        &self,
        object_id: &str,
        body_content: &[u8],
        split_size: usize,
    ) -> Result<(), GitLFSError> {
        let mut sub_ids = vec![];
        for chunk in body_content.chunks(split_size) {
            let sub_id = hex::encode(ring::digest::digest(&ring::digest::SHA256, chunk));
            self.put_object(&sub_id, chunk.to_vec())
                .await
                .expect("Write file failed");
            sub_ids.push(sub_id);
        }
        tracing::debug!(
            "lfs object {} split into {} chunks",
            object_id,
            sub_ids.len()
        );

        // save the relationship to database
        let mut offset = 0;
        let mut save_models = vec![];
        for sub_id in sub_ids {
            let size = min(split_size as i64, body_content.len() as i64 - offset);
            let model = lfs_split_relations::Model {
                ori_oid: object_id.to_owned(),
                sub_oid: sub_id.to_owned(),
                offset,
                size,
            };
            save_models.push(model);
            offset += size;
        }
        let result = self.lfs_db_storage.save_lfs_relations(save_models).await;
        if result.is_err() {
            tracing::error!("lfs object upload failed, failed to save split relationship");
            return Err(GitLFSError::GeneralError(String::from(
                "Header not acceptable!",
            )));
        }
        tracing::debug!("lfs object split relationship saved");
        Ok(())
    }

    async fn exist_object(&self, object_id: &str, enable_split: bool) -> bool {
        if enable_split {
            let relations = self
                .lfs_db_storage
                .get_lfs_relations(object_id)
                .await
                .unwrap();
            if relations.is_empty() {
                return false;
            }
            return relations.iter().all(|relation| {
                exist_object(self.config.lfs_file_path.clone(), &relation.sub_oid)
            });
        } else {
            exist_object(self.config.lfs_file_path.clone(), object_id)
        }
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
    use crate::lfs_storage::{local_storage::LocalStorage, LfsFileStorage};

    #[tokio::test]
    async fn test_content_store() {
        let oid = "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72".to_owned();
        let content = "test content".as_bytes().to_vec();

        let local_storage = LocalStorage::mock();
        assert!(local_storage.put_object(&oid, content).await.is_ok());
        assert!(local_storage.exist_object(&oid, false).await);
    }
}
