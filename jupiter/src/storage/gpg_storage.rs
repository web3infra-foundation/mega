use std::ops::Deref;

use callisto::entity_ext::generate_id;
use callisto::gpg_key;
use common::errors::MegaError;
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::IntoActiveModel;
use sea_orm::ActiveModelTrait;
use sea_orm::QueryFilter;

use crate::storage::base_storage::StorageConnector;
use crate::storage::{base_storage::BaseStorage};

#[derive(Clone)]
pub struct GpgStorage {
    pub base: BaseStorage 
}

impl Deref for GpgStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
       &self.base 
    }
}

impl GpgStorage{
    fn create_sample_key(&self, user_id: i64) -> gpg_key::Model {
        let key = gpg_key::Model {
            id: 123,
            key_id: 123,
            user_id,
            public_key: "PUBKEY".to_string(),
            fingerprint: format!("fp-{}", 123456),
            alias: "sample".to_string(),
            is_verified: false,
            created_at: chrono::Utc::now().naive_utc(),
            expires_at: None,
        };
        key
    }

    pub async fn save_gpg_key(&self, user_id: i64) -> Result<(), MegaError> {
        let key = self.create_sample_key(user_id);
        let a_model = key.into_active_model();
        a_model.insert(self.get_connection()).await?;
        Ok(())
    }

    pub async fn remove_gpg_key(&self, user_id: i64, key_id: i64) -> Result<(), MegaError> {
        gpg_key::Entity::delete_many()
            .filter(gpg_key::Column::UserId.eq(user_id))
            .filter(gpg_key::Column::KeyId.eq(key_id))
            .exec(self.get_connection())
            .await?;
        Ok(())
    }
}

