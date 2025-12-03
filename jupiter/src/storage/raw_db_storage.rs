use std::ops::Deref;

use futures::Stream;
use sea_orm::{ColumnTrait, DbErr, EntityTrait, IntoActiveModel, QueryFilter};

use callisto::raw_blob;
use common::errors::MegaError;
use git_internal::internal::object::blob::Blob;

use crate::storage::base_storage::{BaseStorage, StorageConnector};
use crate::utils::converter::ToRawBlob;

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

    /// Save a raw blob to database
    pub async fn save_raw_blob_from_content(&self, data: Vec<u8>) -> Result<String, MegaError> {
        let blob = Blob::from_content_bytes(data);
        let blob_hash = blob.id.to_string();

        // Use ToRawBlob trait for conversion
        let model = blob.to_raw_blob();

        raw_blob::Entity::insert(model.into_active_model())
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(raw_blob::Column::Sha1)
                    .do_nothing()
                    .to_owned(),
            )
            .do_nothing()
            .exec(self.get_connection())
            .await?;

        Ok(blob_hash)
    }
}
