use std::ops::Deref;

use callisto::vault::*;
use common::errors::MegaError;
use sea_orm::*;
use sea_orm_migration::prelude::OnConflict;

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct VaultStorage {
    pub base: BaseStorage,
}

impl Deref for VaultStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl VaultStorage {
    pub async fn list_keys(&self, prefix: impl AsRef<str>) -> Result<Vec<String>, MegaError> {
        Entity::find()
            .select_column(Column::Key)
            .filter(Column::Key.like(format!("{}%", prefix.as_ref()).as_str()))
            .into_tuple::<String>()
            .all(self.get_connection())
            .await
            .map_err(|e| {
                MegaError::with_message(format!(
                    "Failed to list vault with prefix: {}, {}",
                    prefix.as_ref(),
                    e
                ))
            })
    }

    pub async fn load(&self, key: impl AsRef<str>) -> Result<Option<Model>, MegaError> {
        let found = Entity::find()
            .filter(Column::Key.eq(key.as_ref()))
            .one(self.get_connection())
            .await?;
        Ok(found)
    }

    pub async fn save(&self, key: impl AsRef<str>, value: Vec<u8>) -> Result<(), MegaError> {
        let model = ActiveModel {
            id: NotSet,
            key: Set(key.as_ref().to_string()),
            value: Set(value),
        };
 
        match Entity::insert(model)
            .on_conflict(
                OnConflict::column(Column::Key)
                    .update_column(Column::Value)
                    .to_owned(),
            )
            .exec(self.get_connection())
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(MegaError::with_message(format!(
                "Failed to save vault entry '{}': {}",
                key.as_ref(),
                e
            ))),
        }
    }

    pub async fn delete(&self, key: impl AsRef<str>) -> Result<(), MegaError> {
        let model = Entity::find()
            .filter(Column::Key.eq(key.as_ref()))
            .one(self.get_connection())
            .await?
            .ok_or_else(|| {
                MegaError::with_message(format!("Vault key '{}' not found", key.as_ref()).as_str())
            })?;

        match model.delete(self.get_connection()).await {
            Ok(_) => Ok(()),
            Err(e) => Err(MegaError::with_message(format!(
                "Failed to delete vault entry '{}': {}",
                key.as_ref(),
                e
            ))),
        }
    }
}
