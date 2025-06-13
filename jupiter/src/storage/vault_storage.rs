use std::sync::Arc;

use callisto::vault::*;
use common::errors::MegaError;
use sea_orm::*;

#[derive(Clone)]
pub struct VaultStorage {
    pub connection: Arc<DatabaseConnection>,
}

impl VaultStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>) -> Self {
        VaultStorage { connection }
    }

    pub fn mock() -> Self {
        VaultStorage {
            connection: Arc::new(DatabaseConnection::default()),
        }
    }

    pub async fn list_keys(&self, prefix: impl AsRef<str>) -> Result<Vec<String>, MegaError> {
        Entity::find()
            .order_by_asc(Column::Key.like(format!("{}%", prefix.as_ref()).as_str()))
            .select_column(Column::Key)
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

    pub async fn load(&self, key: impl AsRef<str>) -> Result<Model, MegaError> {
        Entity::find()
            .filter(Column::Key.eq(key.as_ref()))
            .one(self.get_connection())
            .await?
            .ok_or_else(|| {
                MegaError::with_message(format!("Vault key '{}' not found", key.as_ref()).as_str())
            })
    }

    pub async fn save(&self, key: impl AsRef<str>, value: Vec<u8>) -> Result<(), MegaError> {
        let model = Model {
            id: 0,
            key: key.as_ref().to_string(),
            value,
        }
        .into_active_model();

        match model.save(self.get_connection()).await {
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
