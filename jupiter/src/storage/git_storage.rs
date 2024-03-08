use std::{env, io::Cursor, sync::Arc};

use async_trait::async_trait;

use common::errors::MegaError;
use venus::{
    hash::SHA1,
    internal::{
        object::{types::ObjectType, utils},
        pack::{entry::Entry, reference::RefCommand},
        repo::Repo,
    },
};

use crate::{
    raw_storage::{self, RawStorage},
    storage::GitStorageProvider,
};

pub struct GitStorage {
    pub raw_storage: Arc<dyn RawStorage>,
}

#[async_trait]
impl GitStorageProvider for GitStorage {
    async fn save_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError> {
        self.raw_storage
            .put_ref(&repo.repo_name, &refs.ref_name, &refs.new_id)
            .await
    }

    async fn remove_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError> {
        self.raw_storage
            .delete_ref(&repo.repo_name, &refs.ref_name)
            .await
    }

    async fn get_ref(&self, repo: &Repo, ref_name: &str) -> Result<String, MegaError> {
        self.raw_storage.get_ref(&repo.repo_name, ref_name).await
    }

    async fn update_ref(&self, repo: &Repo, ref_name: &str, new_id: &str) -> Result<(), MegaError> {
        self.raw_storage
            .update_ref(&repo.repo_name, ref_name, new_id)
            .await
    }

    async fn save_entry(&self, repo: Repo, result_entity: Vec<Entry>) -> Result<(), MegaError> {
        for entry in result_entity {
            self.raw_storage
                .put_object(
                    &repo.repo_name,
                    &entry.hash.to_plain_str(),
                    &entry.data,
                )
                .await
                .unwrap();
        }
        Ok(())
    }

    async fn get_entry_by_sha1(
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

impl GitStorage {
    pub async fn new() -> Self {
        let storage_type = env::var("MEGA_RAW_STORAGE").unwrap();
        let path = env::var("MEGA_OBJ_LOCAL_PATH").unwrap();
        GitStorage {
            raw_storage: raw_storage::init(storage_type, path).await,
        }
    }

    pub fn mock() -> Self {
        Self {
            raw_storage: raw_storage::mock()
        }
    }
}
