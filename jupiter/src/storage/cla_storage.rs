use std::ops::Deref;

use callisto::cla_sign_status;
use common::errors::MegaError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QuerySelect, Set,
};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone, Debug)]
pub struct ClaStorage {
    pub base: BaseStorage,
}

impl Deref for ClaStorage {
    type Target = BaseStorage;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl ClaStorage {
    pub async fn get_status(
        &self,
        username: &str,
    ) -> Result<Option<cla_sign_status::Model>, MegaError> {
        Ok(cla_sign_status::Entity::find_by_id(username.to_string())
            .one(self.get_connection())
            .await?)
    }

    pub async fn get_or_create_status(
        &self,
        username: &str,
    ) -> Result<cla_sign_status::Model, MegaError> {
        if let Some(model) = self.get_status(username).await? {
            return Ok(model);
        }

        let now = chrono::Utc::now().naive_utc();
        let model = cla_sign_status::ActiveModel {
            username: Set(username.to_string()),
            cla_signed: Set(false),
            cla_signed_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        Ok(model.insert(self.get_connection()).await?)
    }

    pub async fn is_signed(&self, username: &str) -> Result<bool, MegaError> {
        Ok(self
            .get_status(username)
            .await?
            .map(|status| status.cla_signed)
            .unwrap_or(false))
    }

    pub async fn sign(&self, username: &str) -> Result<cla_sign_status::Model, MegaError> {
        let now = chrono::Utc::now().naive_utc();

        if let Some(model) = self.get_status(username).await? {
            if model.cla_signed {
                return Ok(model);
            }

            let mut active_model = model.into_active_model();
            active_model.cla_signed = Set(true);
            active_model.cla_signed_at = Set(Some(now));
            active_model.updated_at = Set(now);
            return Ok(active_model.update(self.get_connection()).await?);
        }

        let active_model = cla_sign_status::ActiveModel {
            username: Set(username.to_string()),
            cla_signed: Set(true),
            cla_signed_at: Set(Some(now)),
            created_at: Set(now),
            updated_at: Set(now),
        };

        Ok(active_model.insert(self.get_connection()).await?)
    }

    pub async fn unsigned_users(&self, usernames: &[String]) -> Result<Vec<String>, MegaError> {
        if usernames.is_empty() {
            return Ok(Vec::new());
        }

        let signed_users: Vec<String> = cla_sign_status::Entity::find()
            .select_only()
            .column(cla_sign_status::Column::Username)
            .filter(cla_sign_status::Column::Username.is_in(usernames.iter().cloned()))
            .filter(cla_sign_status::Column::ClaSigned.eq(true))
            .into_tuple::<String>()
            .all(self.get_connection())
            .await?;

        let unsigned = usernames
            .iter()
            .filter(|username| !signed_users.contains(*username))
            .cloned()
            .collect();

        Ok(unsigned)
    }
}
