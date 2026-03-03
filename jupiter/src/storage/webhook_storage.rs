use std::ops::Deref;

use callisto::{mega_webhook, mega_webhook_delivery};
use common::errors::MegaError;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct WebhookStorage {
    pub base: BaseStorage,
}

impl Deref for WebhookStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl WebhookStorage {
    pub async fn create_webhook(
        &self,
        model: mega_webhook::Model,
    ) -> Result<mega_webhook::Model, MegaError> {
        let res = model
            .into_active_model()
            .insert(self.get_connection())
            .await?;
        Ok(res)
    }

    pub async fn list_webhooks(&self) -> Result<Vec<mega_webhook::Model>, MegaError> {
        let models = mega_webhook::Entity::find()
            .all(self.get_connection())
            .await?;
        Ok(models)
    }

    pub async fn get_webhook(&self, id: i64) -> Result<Option<mega_webhook::Model>, MegaError> {
        let model = mega_webhook::Entity::find_by_id(id)
            .one(self.get_connection())
            .await?;
        Ok(model)
    }

    pub async fn delete_webhook(&self, id: i64) -> Result<(), MegaError> {
        mega_webhook::Entity::delete_by_id(id)
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    pub async fn find_matching_webhooks(
        &self,
        event_type: &str,
        path: &str,
    ) -> Result<Vec<mega_webhook::Model>, MegaError> {
        let all = mega_webhook::Entity::find()
            .filter(mega_webhook::Column::Active.eq(true))
            .all(self.get_connection())
            .await?;

        let matching = all
            .into_iter()
            .filter(|w| {
                let events: Vec<String> = serde_json::from_str(&w.event_types).unwrap_or_default();
                if !events.iter().any(|e| e == event_type || e == "*") {
                    return false;
                }
                if let Some(ref filter) = w.path_filter {
                    path.starts_with(filter.as_str())
                } else {
                    true
                }
            })
            .collect();

        Ok(matching)
    }

    pub async fn save_delivery(
        &self,
        model: mega_webhook_delivery::Model,
    ) -> Result<(), MegaError> {
        model
            .into_active_model()
            .insert(self.get_connection())
            .await?;
        Ok(())
    }
}
