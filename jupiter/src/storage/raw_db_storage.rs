use std::sync::Arc;

use futures::Stream;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

use callisto::raw_blob;
use common::errors::MegaError;

#[derive(Clone)]
pub struct RawDbStorage {
    pub connection: Arc<DatabaseConnection>,
}

impl RawDbStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>) -> Self {
        RawDbStorage { connection }
    }

    pub fn mock() -> Self {
        RawDbStorage {
            connection: Arc::new(DatabaseConnection::default()),
        }
    }

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
