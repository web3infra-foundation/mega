use crate::driver::MegaError;
use crate::driver::ObjectStorage;
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct MysqlStorage {
    pub connection: DatabaseConnection,
}

impl MysqlStorage {
    pub fn new(connection: DatabaseConnection) -> MysqlStorage {
        MysqlStorage { connection }
    }
}

#[async_trait]
impl ObjectStorage for MysqlStorage {
    async fn get_head_object_id(&self, _path: &Path) -> String {
        todo!()
    }

    async fn get_ref_object_id(&self, _path: &Path) -> HashMap<String, String> {
        todo!()
    }

    async fn get_full_pack_data(&self, _repo_path: &Path) -> Result<Vec<u8>, MegaError> {
        todo!()
    }

    async fn get_incremental_pack_data(
        &self,
        _repo_path: &Path,
        _want: &HashSet<String>,
        _have: &HashSet<String>,
    ) -> Result<Vec<u8>, MegaError> {
        todo!()
    }

    async fn get_commit_by_hash(&self, _hash: &str) -> Result<Vec<u8>, MegaError> {
        todo!()
    }

    async fn get_hash_object(&self, _hash: &str) -> Result<Vec<u8>, MegaError> {
        todo!()
    }
}
