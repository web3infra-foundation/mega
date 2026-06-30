//! Webhook CRUD for the mono HTTP API.

use api_model::common::Pagination;
use common::errors::MegaError;
use jupiter::{
    idgenerator::IdInstance,
    service::webhook_service::{encrypt_webhook_secret, validate_webhook_target_url},
    storage::webhook_storage::WebhookWithEventTypes,
};

use crate::{
    api_service::mono::MonoApiService,
    model::webhook::{CreateWebhookRequest, WebhookResponse, parse_webhook_event_types},
};

impl MonoApiService {
    pub async fn create_webhook(
        &self,
        payload: CreateWebhookRequest,
    ) -> Result<WebhookResponse, MegaError> {
        validate_webhook_target_url(&payload.target_url)
            .map_err(|e| MegaError::Other(e.to_string()))?;
        if payload.secret.is_empty() {
            return Err(MegaError::Other(
                "webhook secret cannot be empty".to_string(),
            ));
        }

        let encrypted_secret = encrypt_webhook_secret(&payload.secret)?;
        let event_types =
            parse_webhook_event_types(payload.event_types).map_err(MegaError::Other)?;

        let now = chrono::Utc::now().naive_utc();
        let model = callisto::mega_webhook::Model {
            id: IdInstance::next_id(),
            target_url: payload.target_url,
            secret: encrypted_secret,
            event_types: serde_json::to_string(&event_types).unwrap_or_else(|_| "[]".to_string()),
            path_filter: payload.path_filter,
            active: payload.active.unwrap_or(true),
            created_at: now,
            updated_at: now,
        };

        let created: WebhookWithEventTypes = self
            .storage
            .webhook_storage()
            .create_webhook(model, event_types)
            .await?;
        Ok(created.into())
    }

    pub async fn list_webhooks(
        &self,
        pagination: Pagination,
    ) -> Result<(Vec<WebhookResponse>, u64), MegaError> {
        let (webhooks, total) = self
            .storage
            .webhook_storage()
            .list_webhooks(pagination)
            .await?;
        Ok((webhooks.into_iter().map(|w| w.into()).collect(), total))
    }

    pub async fn delete_webhook(&self, id: i64) -> Result<(), MegaError> {
        self.storage.webhook_storage().delete_webhook(id).await
    }
}
