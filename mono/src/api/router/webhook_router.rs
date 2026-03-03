use api_model::common::CommonResult;
use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use idgenerator::IdInstance;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{MonoApiServiceState, error::ApiError},
    server::http_server::WEBHOOK_TAG,
};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new()
        .routes(routes!(create_webhook))
        .routes(routes!(list_webhooks))
        .routes(routes!(delete_webhook))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWebhookRequest {
    pub target_url: String,
    pub secret: String,
    pub event_types: Vec<String>,
    pub path_filter: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookResponse {
    pub id: i64,
    pub target_url: String,
    pub event_types: Vec<String>,
    pub path_filter: Option<String>,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<callisto::mega_webhook::Model> for WebhookResponse {
    fn from(m: callisto::mega_webhook::Model) -> Self {
        let event_types: Vec<String> = serde_json::from_str(&m.event_types).unwrap_or_default();
        Self {
            id: m.id,
            target_url: m.target_url,
            event_types,
            path_filter: m.path_filter,
            active: m.active,
            created_at: m.created_at.to_string(),
            updated_at: m.updated_at.to_string(),
        }
    }
}

/// Create a webhook
#[utoipa::path(
    post,
    path = "/webhooks",
    request_body = CreateWebhookRequest,
    responses(
        (status = 200, body = CommonResult<WebhookResponse>, content_type = "application/json")
    ),
    tag = WEBHOOK_TAG
)]
async fn create_webhook(
    state: State<MonoApiServiceState>,
    Json(payload): Json<CreateWebhookRequest>,
) -> Result<Json<CommonResult<WebhookResponse>>, ApiError> {
    let now = Utc::now().naive_utc();
    let model = callisto::mega_webhook::Model {
        id: IdInstance::next_id(),
        target_url: payload.target_url,
        secret: payload.secret,
        event_types: serde_json::to_string(&payload.event_types)
            .unwrap_or_else(|_| "[]".to_string()),
        path_filter: payload.path_filter,
        active: payload.active.unwrap_or(true),
        created_at: now,
        updated_at: now,
    };

    let created = state.webhook_stg().create_webhook(model).await?;
    Ok(Json(CommonResult::success(Some(created.into()))))
}

/// List all webhooks
#[utoipa::path(
    get,
    path = "/webhooks",
    responses(
        (status = 200, body = CommonResult<Vec<WebhookResponse>>, content_type = "application/json")
    ),
    tag = WEBHOOK_TAG
)]
async fn list_webhooks(
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<WebhookResponse>>>, ApiError> {
    let webhooks = state.webhook_stg().list_webhooks().await?;
    let res: Vec<WebhookResponse> = webhooks.into_iter().map(|w| w.into()).collect();
    Ok(Json(CommonResult::success(Some(res))))
}

/// Delete a webhook
#[utoipa::path(
    delete,
    params(("id", description = "Webhook ID")),
    path = "/webhooks/{id}",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = WEBHOOK_TAG
)]
async fn delete_webhook(
    state: State<MonoApiServiceState>,
    Path(id): Path<i64>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state.webhook_stg().delete_webhook(id).await?;
    Ok(Json(CommonResult::success(None)))
}
