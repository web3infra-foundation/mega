use std::{io::Cursor, sync::Arc};

use async_trait::async_trait;

use common::errors::MegaError;
use venus::{
    hash::SHA1,
    internal::{
        object::{types::ObjectType, utils},
        pack::{entry::Entry, header::EntryHeader, reference::RefCommand},
        repo::Repo,
    },
};

use crate::{
    raw_storage::{self, RawStorage},
    storage::StorageProvider,
};

pub struct GitStorage {
    pub rawobj_storage: Arc<dyn RawStorage>,
}

#[async_trait]
impl StorageProvider for GitStorage {
    async fn save_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError> {
        self.rawobj_storage
            .put_ref(&repo.repo_name, &refs.ref_name, &refs.new_id)
            .await
    }

    async fn remove_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError> {
        self.rawobj_storage
            .delete_ref(&repo.repo_name, &refs.ref_name)
            .await
    }

    async fn get_ref(&self, repo: Repo, refs: RefCommand) -> Result<String, MegaError> {
        self.rawobj_storage
            .get_ref(&repo.repo_name, &refs.ref_name)
            .await
    }

    async fn update_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError> {
        self.rawobj_storage
            .update_ref(&repo.repo_name, &refs.ref_name, &refs.new_id)
            .await
    }

    async fn save_entry(&self, repo: Repo, result_entity: Vec<Entry>) -> Result<(), MegaError> {
        for entry in result_entity {
            self.rawobj_storage
                .put_object(
                    &repo.repo_name,
                    &entry.hash.unwrap().to_plain_str(),
                    &entry.data,
                )
                .await
                .unwrap();
        }
        Ok(())
    }

    async fn get_entry_by_sha1(&self, repo: Repo, sha1_vec: Vec<&str>) -> Result<Vec<Entry>, MegaError> {
        let mut res: Vec<Entry> = Vec::new();
        for sha1 in sha1_vec {
            let data = self.rawobj_storage.get_object(&repo.repo_name, sha1).await.unwrap();
            let (type_num, _) = utils::read_type_and_size(&mut Cursor::new(&data)).unwrap();
            let o_type = ObjectType::from_u8(type_num).unwrap();
            let header = EntryHeader::from_string(&o_type.to_string());
            let sha1 = SHA1::new(&data.to_vec());
            res.push(Entry {
                header,
                offset: 0,
                data: data.to_vec(),
                hash: Some(sha1),
            })
        }
        Ok(res)
    }
}

impl GitStorage {
    pub async fn new() -> Self {
        GitStorage {
            rawobj_storage: raw_storage::init().await,
        }
    }
}
