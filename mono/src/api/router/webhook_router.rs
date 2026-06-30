use api_model::common::{CommonPage, CommonResult, Pagination};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use callisto::sea_orm_active_enums::WebhookEventTypeEnum;
use ceres::model::webhook::{
    CreateWebhookRequest, ListWebhooksQuery, WebhookResponse, parse_webhook_event_types,
};
use chrono::Utc;
use jupiter::{
    idgenerator::IdInstance,
    service::webhook_service::{encrypt_webhook_secret, validate_webhook_target_url},
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::{MonoApiServiceState, api_doc::WEBHOOK_TAG, error::ApiError};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new()
        .routes(routes!(create_webhook))
        .routes(routes!(list_webhooks))
        .routes(routes!(delete_webhook))
}

fn parse_event_types(raw: Vec<String>) -> Result<Vec<WebhookEventTypeEnum>, ApiError> {
    parse_webhook_event_types(raw).map_err(|s| ApiError::bad_request(anyhow::anyhow!(s)))
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
    validate_webhook_target_url(&payload.target_url).map_err(ApiError::bad_request)?;
    if payload.secret.is_empty() {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "webhook secret cannot be empty"
        )));
    }
    let encrypted_secret = encrypt_webhook_secret(&payload.secret).map_err(ApiError::internal)?;
    let event_types = parse_event_types(payload.event_types)?;

    let now = Utc::now().naive_utc();
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

    let created = state
        .webhook_stg()
        .create_webhook(model, event_types)
        .await?;
    Ok(Json(CommonResult::success(Some(created.into()))))
}

/// List webhooks
#[utoipa::path(
    get,
    path = "/webhooks",
    params(
        ("page" = Option<u64>, Query, description = "Page number, starts from 1. Default: 1"),
        ("per_page" = Option<u64>, Query, description = "Items per page. Default: 20")
    ),
    responses(
        (status = 200, body = CommonResult<CommonPage<WebhookResponse>>, content_type = "application/json")
    ),
    tag = WEBHOOK_TAG
)]
async fn list_webhooks(
    state: State<MonoApiServiceState>,
    Query(query): Query<ListWebhooksQuery>,
) -> Result<Json<CommonResult<CommonPage<WebhookResponse>>>, ApiError> {
    let pagination = build_webhook_pagination(query)?;
    let (webhooks, total) = state.webhook_stg().list_webhooks(pagination).await?;
    let items: Vec<WebhookResponse> = webhooks.into_iter().map(|w| w.into()).collect();
    Ok(Json(CommonResult::success(Some(CommonPage {
        total,
        items,
    }))))
}

/// Delete a webhook
#[utoipa::path(
    delete,
    params(("id", description = "Webhook ID")),
    path = "/webhooks/{id}",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json"),
        (status = 404, description = "Webhook not found")
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

fn build_webhook_pagination(query: ListWebhooksQuery) -> Result<Pagination, ApiError> {
    let mut pagination = Pagination::default();
    if let Some(page) = query.page {
        pagination.page = page;
    }
    if let Some(per_page) = query.per_page {
        pagination.per_page = per_page;
    }

    if pagination.page == 0 {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "page must be greater than or equal to 1"
        )));
    }
    if pagination.per_page == 0 {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "per_page must be greater than or equal to 1"
        )));
    }

    Ok(pagination)
}
