use std::{io::Cursor, sync::Arc};

use async_trait::async_trait;

use callisto::import_refs;
use common::{config::StorageConfig, errors::MegaError};
use mercury::{
    hash::SHA1,
    internal::{
        object::{types::ObjectType, utils},
        pack::entry::Entry,
    },
};

use crate::{
    raw_storage::{self, RawStorage},
    storage::GitStorageProvider,
};

pub struct GitFsStorage {
    pub raw_storage: Arc<dyn RawStorage>,
}

#[async_trait]
impl GitStorageProvider for GitFsStorage {
    async fn save_ref(&self, repo_id: i64, refs: import_refs::Model) -> Result<(), MegaError> {
        self.raw_storage
            .put_ref(repo_id, &refs.ref_name, &refs.ref_git_id)
            .await
    }

    async fn remove_ref(&self, repo_id: i64, ref_name: &str) -> Result<(), MegaError> {
        self.raw_storage.delete_ref(repo_id, ref_name).await
    }

    async fn get_ref(&self, _repo_id: i64) -> Result<Vec<import_refs::Model>, MegaError> {
        // let ref_hash = self.raw_storage.get_ref(&repo.repo_name, ref_name).await;
        // if let Some()
        todo!()
    }

    async fn update_ref(
        &self,
        repo_id: i64,
        ref_name: &str,
        new_id: &str,
    ) -> Result<(), MegaError> {
        self.raw_storage.update_ref(repo_id, ref_name, new_id).await
    }
}

impl GitFsStorage {
    pub async fn new(config: StorageConfig) -> Self {
        GitFsStorage {
            raw_storage: raw_storage::init(config.raw_obj_storage_type, config.raw_obj_local_path)
                .await,
        }
    }

    pub fn mock() -> Self {
        Self {
            raw_storage: raw_storage::mock(),
        }
    }

    pub async fn save_entry(&self, repo_id: i64, entry_list: Vec<Entry>) -> Result<(), MegaError> {
        for entry in entry_list {
            self.raw_storage
                .put_object(repo_id, &entry.hash.to_plain_str(), &entry.data)
                .await
                .unwrap();
        }
        Ok(())
    }

    pub async fn get_entry_by_sha1(
        &self,
        repo_id: i64,
        sha1_vec: Vec<&str>,
    ) -> Result<Vec<Entry>, MegaError> {
        let mut res: Vec<Entry> = Vec::new();
        for sha1 in sha1_vec {
            let data = self
                .raw_storage
                .get_object(repo_id, sha1)
                .await
                .unwrap();
            let (type_num, _) = utils::read_type_and_size(&mut Cursor::new(&data)).unwrap();
            let obj_type = ObjectType::from_u8(type_num).unwrap();
            let hash = SHA1::new(&data.to_vec());
            res.push(Entry {
                obj_type,
                data: data.to_vec(),
                hash,
            })
        }
        Ok(res)
    }
}
