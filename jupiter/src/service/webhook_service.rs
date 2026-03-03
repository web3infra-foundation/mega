use std::time::Duration;

use callisto::mega_cl;
use chrono::Utc;
use hmac::{Hmac, Mac};
use idgenerator::IdInstance;
use serde::Serialize;
use sha2::Sha256;

use crate::storage::webhook_storage::WebhookStorage;

pub mod events {
    pub const CL_CREATED: &str = "cl.created";
    pub const CL_UPDATED: &str = "cl.updated";
    pub const CL_MERGED: &str = "cl.merged";
    pub const CL_CLOSED: &str = "cl.closed";
    pub const CL_REOPENED: &str = "cl.reopened";
    pub const CL_COMMENT_CREATED: &str = "cl.comment.created";
}

#[derive(Debug, Clone, Serialize)]
pub struct WebhookPayload {
    pub mega_version: String,
    pub event: String,
    pub timestamp: String,
    pub cl: ClPayload,
    pub repository: RepositoryPayload,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepositoryPayload {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthorPayload {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClPayload {
    pub id: i64,
    pub link: String,
    pub title: String,
    pub author: AuthorPayload,
    pub status: String,
    pub base_branch: String,
    pub head_commit: String,
}

impl From<&mega_cl::Model> for ClPayload {
    fn from(model: &mega_cl::Model) -> Self {
        Self {
            id: model.id,
            link: model.link.clone(),
            title: model.title.clone(),
            author: AuthorPayload {
                name: model.username.clone(),
            },
            status: format!("{:?}", model.status),
            base_branch: "main".to_string(),
            head_commit: model.to_hash.clone(),
        }
    }
}

#[derive(Clone)]
pub struct WebhookService {
    storage: WebhookStorage,
    client: reqwest::Client,
}

impl WebhookService {
    pub fn new(storage: WebhookStorage) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build reqwest client");
        Self { storage, client }
    }

    pub fn mock(storage: WebhookStorage) -> Self {
        Self {
            storage,
            client: reqwest::Client::new(),
        }
    }

    pub fn dispatch(&self, event_type: &str, cl_model: &mega_cl::Model) {
        let svc = self.clone();
        let event_type = event_type.to_string();
        let cl_payload = ClPayload::from(cl_model);
        let path = cl_model.path.clone();

        tokio::spawn(async move {
            if let Err(e) = svc.dispatch_inner(&event_type, cl_payload, &path).await {
                tracing::error!("webhook dispatch error: {e}");
            }
        });
    }

    async fn dispatch_inner(
        &self,
        event_type: &str,
        cl_payload: ClPayload,
        path: &str,
    ) -> Result<(), common::errors::MegaError> {
        let webhooks = self
            .storage
            .find_matching_webhooks(event_type, path)
            .await?;

        for webhook in webhooks {
            let repo_name = path
                .trim_start_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or(path)
                .to_string();
            let payload = WebhookPayload {
                mega_version: env!("CARGO_PKG_VERSION").to_string(),
                event: event_type.to_string(),
                timestamp: Utc::now().to_rfc3339(),
                cl: cl_payload.clone(),
                repository: RepositoryPayload {
                    path: path.to_string(),
                    name: repo_name,
                },
            };

            let payload_json = serde_json::to_string(&payload)?;

            let mut last_err = None;
            for attempt in 1..=3 {
                match self
                    .deliver(
                        &webhook.target_url,
                        &webhook.secret,
                        event_type,
                        &payload_json,
                    )
                    .await
                {
                    Ok((status, body)) => {
                        let delivery = callisto::mega_webhook_delivery::Model {
                            id: IdInstance::next_id(),
                            webhook_id: webhook.id,
                            event_type: event_type.to_string(),
                            payload: payload_json.clone(),
                            response_status: Some(status as i32),
                            response_body: Some(body),
                            success: (200..300).contains(&status),
                            attempt,
                            error_message: None,
                            created_at: Utc::now().naive_utc(),
                        };
                        let success = delivery.success;
                        if let Err(e) = self.storage.save_delivery(delivery).await {
                            tracing::warn!("failed to save webhook delivery record: {e}");
                        }
                        if success {
                            break;
                        }
                    }
                    Err(e) => {
                        let delivery = callisto::mega_webhook_delivery::Model {
                            id: IdInstance::next_id(),
                            webhook_id: webhook.id,
                            event_type: event_type.to_string(),
                            payload: payload_json.clone(),
                            response_status: None,
                            response_body: None,
                            success: false,
                            attempt,
                            error_message: Some(e.to_string()),
                            created_at: Utc::now().naive_utc(),
                        };
                        if let Err(save_err) = self.storage.save_delivery(delivery).await {
                            tracing::warn!("failed to save webhook delivery record: {save_err}");
                        }
                        last_err = Some(e);
                    }
                }

                if attempt < 3 {
                    let backoff = Duration::from_secs(2u64.pow(attempt as u32));
                    tokio::time::sleep(backoff).await;
                }
            }

            if let Some(e) = last_err {
                tracing::warn!(
                    "webhook delivery failed after 3 attempts for webhook_id={}: {e}",
                    webhook.id
                );
            }
        }

        Ok(())
    }

    async fn deliver(
        &self,
        url: &str,
        secret: &str,
        event_type: &str,
        payload: &str,
    ) -> Result<(u16, String), common::errors::MegaError> {
        let signature = compute_hmac_signature(secret, payload);

        let resp = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Mega-Event", event_type)
            .header("X-Mega-Signature", format!("sha256={signature}"))
            .body(payload.to_string())
            .send()
            .await
            .map_err(|e| common::errors::MegaError::Other(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        Ok((status, body))
    }
}

fn compute_hmac_signature(secret: &str, payload: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
