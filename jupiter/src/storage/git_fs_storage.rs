use std::{io::Cursor, sync::Arc};

use async_trait::async_trait;

use common::{config::StorageConfig, errors::MegaError};
use mercury::{
    hash::SHA1,
    internal::{
        object::{types::ObjectType, utils},
        pack::entry::Entry,
    },
};
use venus::{
    import_repo::import_refs::{RefCommand, Refs},
    import_repo::repo::Repo,
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
    async fn save_ref(&self, repo: &Repo, refs: &RefCommand) -> Result<(), MegaError> {
        self.raw_storage
            .put_ref(&repo.repo_name, &refs.ref_name, &refs.new_id)
            .await
    }

    async fn remove_ref(&self, repo: &Repo, refs: &RefCommand) -> Result<(), MegaError> {
        self.raw_storage
            .delete_ref(&repo.repo_name, &refs.ref_name)
            .await
    }

    async fn get_ref(&self, _repo: &Repo) -> Result<Vec<Refs>, MegaError> {
        // let ref_hash = self.raw_storage.get_ref(&repo.repo_name, ref_name).await;
        // if let Some()
        todo!()
    }

    async fn update_ref(&self, repo: &Repo, ref_name: &str, new_id: &str) -> Result<(), MegaError> {
        self.raw_storage
            .update_ref(&repo.repo_name, ref_name, new_id)
            .await
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

    pub async fn save_entry(&self, repo: &Repo, entry_list: Vec<Entry>) -> Result<(), MegaError> {
        for entry in entry_list {
            self.raw_storage
                .put_object(&repo.repo_name, &entry.hash.to_plain_str(), &entry.data)
                .await
                .unwrap();
        }
        Ok(())
    }

    pub async fn get_entry_by_sha1(
        &self,
        repo: Repo,
        sha1_vec: Vec<&str>,
    ) -> Result<Vec<Entry>, MegaError> {
        let mut res: Vec<Entry> = Vec::new();
        for sha1 in sha1_vec {
            let data = self
                .raw_storage
                .get_object(&repo.repo_name, sha1)
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
