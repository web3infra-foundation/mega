use api_model::common::CommonResult;
use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Duration, Utc};
use sea_orm::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{
        MonoApiServiceState, api_common::group_permission::ensure_admin, error::ApiError,
        oauth::model::LoginUser,
    },
    server::http_server::BOT_TAG,
};

/// Request body for creating a new bot token.
#[derive(Deserialize, ToSchema)]
pub struct CreateBotTokenRequest {
    /// Human-readable token name for identification.
    pub token_name: String,
    /// Optional relative expiry in seconds from now.
    pub expires_in: Option<i64>,
}

/// Response body when a bot token is created.
///
/// Note: `token_plain` is only returned once and is never stored in plaintext.
#[derive(Serialize, ToSchema)]
pub struct CreateBotTokenResponse {
    pub id: i64,
    pub token_name: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub token_plain: String,
}

/// Item in the list bot tokens response.
#[derive(Serialize, ToSchema)]
pub struct ListBotTokenItem {
    pub id: i64,
    pub token_name: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
}

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/bots",
        OpenApiRouter::new()
            .routes(routes!(create_bot_token))
            .routes(routes!(list_bot_tokens))
            .routes(routes!(revoke_bot_token))
            .routes(routes!(revoke_all_bot_tokens)),
    )
}

/// POST /api/v1/bots/{bot_id}/tokens
///
/// Create a new bot token. Only admins can perform this operation.
#[utoipa::path(
    post,
    path = "/{bot_id}/tokens",
    request_body = CreateBotTokenRequest,
    params(
        ("bot_id" = i64, Path, description = "Bot ID")
    ),
    responses(
        (status = 200, body = CommonResult<CreateBotTokenResponse>),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "Bot not found"),
    ),
    tag = BOT_TAG
)]
async fn create_bot_token(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path(bot_id): Path<i64>,
    Json(req): Json<CreateBotTokenRequest>,
) -> Result<Json<CommonResult<CreateBotTokenResponse>>, ApiError> {
    ensure_admin(&state, &user).await?;

    let expires_at: Option<DateTimeWithTimeZone> = req.expires_in.map(|seconds| {
        let when = Utc::now() + Duration::seconds(seconds);
        DateTimeWithTimeZone::from(when)
    });

    let (model, token_plain) = state
        .storage
        .bots_storage()
        .generate_bot_token(bot_id, &req.token_name, expires_at)
        .await?;

    let resp = CreateBotTokenResponse {
        id: model.id,
        token_name: model.token_name,
        expires_at: model.expires_at.map(|dt| dt.with_timezone(&Utc)),
        token_plain,
    };

    Ok(Json(CommonResult::success(Some(resp))))
}

/// GET /api/v1/bots/{bot_id}/tokens
///
/// List existing tokens for a bot (without plaintext).
#[utoipa::path(
    get,
    path = "/{bot_id}/tokens",
    params(
        ("bot_id" = i64, Path, description = "Bot ID")
    ),
    responses(
        (status = 200, body = CommonResult<Vec<ListBotTokenItem>>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "Bot not found"),
    ),
    tag = BOT_TAG
)]
async fn list_bot_tokens(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path(bot_id): Path<i64>,
) -> Result<Json<CommonResult<Vec<ListBotTokenItem>>>, ApiError> {
    ensure_admin(&state, &user).await?;

    let tokens = state.storage.bots_storage().list_bot_tokens(bot_id).await?;

    let items = tokens
        .into_iter()
        .map(|t| ListBotTokenItem {
            id: t.id,
            token_name: t.token_name,
            expires_at: t.expires_at.map(|dt| dt.with_timezone(&Utc)),
            revoked: t.revoked,
            created_at: t.created_at.with_timezone(&Utc),
        })
        .collect();

    Ok(Json(CommonResult::success(Some(items))))
}

/// DELETE /api/v1/bots/{bot_id}/tokens/{id}
///
/// Revoke a single bot token. Idempotent.
#[utoipa::path(
    delete,
    path = "/{bot_id}/tokens/{id}",
    params(
        ("bot_id" = i64, Path, description = "Bot ID"),
        ("id" = i64, Path, description = "Token ID")
    ),
    responses(
        (status = 200, body = CommonResult<()>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "Bot or token not found"),
    ),
    tag = BOT_TAG
)]
async fn revoke_bot_token(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path((bot_id, token_id)): Path<(i64, i64)>,
) -> Result<Json<CommonResult<()>>, ApiError> {
    ensure_admin(&state, &user).await?;

    state
        .storage
        .bots_storage()
        .revoke_bot_token(bot_id, token_id)
        .await?;

    Ok(Json(CommonResult::success(None)))
}

/// POST /api/v1/bots/{bot_id}/tokens/revoke_all
///
/// Revoke all tokens for a given bot. Idempotent.
#[utoipa::path(
    post,
    path = "/{bot_id}/tokens/revoke_all",
    params(
        ("bot_id" = i64, Path, description = "Bot ID")
    ),
    responses(
        (status = 200, body = CommonResult<()>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "Bot not found"),
    ),
    tag = BOT_TAG
)]
async fn revoke_all_bot_tokens(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path(bot_id): Path<i64>,
) -> Result<Json<CommonResult<()>>, ApiError> {
    ensure_admin(&state, &user).await?;

    state
        .storage
        .bots_storage()
        .revoke_bot_tokens_by_bot(bot_id)
        .await?;

    Ok(Json(CommonResult::success(None)))
}
