use std::ops::Deref;
use std::str::FromStr;
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
    fn create_key(&self, user_id: i64, gpg_content: String, expires_days: Option<i32>) -> Result<gpg_key::Model, MegaError> {
        let cert = sequoia_openpgp::Cert::from_str(&gpg_content)?;
        let fingerprint = cert.fingerprint().to_hex();
        let key_id = format!("{:016X}", cert.keyid());
        let created_at = chrono::Utc::now().naive_utc();
        let expires_at = expires_days.map(|days| created_at+ chrono::Duration::days(days as i64));

        let key = gpg_key::Model {
            id: generate_id(),
            user_id,
            key_id,
            public_key: gpg_content,
            fingerprint,
            alias: "user-key".to_string(),
            created_at,
            expires_at
        };

        Ok(key)
    }

    pub async fn add_gpg_key(&self, user_id: i64, gpg_content: String, expired_at: Option<i32>) -> Result<(), MegaError> {
        let key = self.create_key(user_id, gpg_content, expired_at)?;
        let a_model = key.into_active_model();
        a_model.insert(self.get_connection()).await?;
        Ok(())
    }

    pub async fn save_gpg_key(&self, key: gpg_key::Model) -> Result<(), MegaError> {
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

