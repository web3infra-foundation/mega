use std::ops::Deref;

use futures::Stream;
use sea_orm::{ColumnTrait, DbErr, EntityTrait, QueryFilter};

use callisto::raw_blob;
use common::errors::MegaError;

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct RawDbStorage {
    pub base: BaseStorage,
}

impl Deref for RawDbStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl RawDbStorage {
    pub async fn get_raw_blobs_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<raw_blob::Model>, MegaError> {
        Ok(raw_blob::Entity::find()
            .filter(raw_blob::Column::Sha1.is_in(hashes))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_raw_blob_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<raw_blob::Model>, MegaError> {
        Ok(raw_blob::Entity::find()
            .filter(raw_blob::Column::Sha1.eq(hash))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_raw_blobs_stream(
        &self,
        hashes: Vec<String>,
    ) -> Result<impl Stream<Item = Result<raw_blob::Model, DbErr>> + '_ + Send, MegaError> {
        Ok(raw_blob::Entity::find()
            .filter(raw_blob::Column::Sha1.is_in(hashes))
            .stream(self.get_connection())
            .await
            .unwrap())
    }
}
