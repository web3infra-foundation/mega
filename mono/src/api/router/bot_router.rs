use anyhow::anyhow;
use api_model::common::CommonResult;
use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::bots::{BotRes, ChangeInstallationStatus, InstallBotReq, InstallationTargetType};
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

/// Maximum allowed expires_in in seconds (10 years).
const MAX_EXPIRES_IN_SECS: i64 = 365 * 24 * 3600 * 10;
/// Minimum allowed expires_in in seconds.
const MIN_EXPIRES_IN_SECS: i64 = 1;

async fn ensure_bot_exists(state: &MonoApiServiceState, bot_id: i64) -> Result<(), ApiError> {
    let bot = state
        .storage
        .bots_storage()
        .get_bot_by_id(bot_id)
        .await
        .map_err(ApiError::from)?;
    if bot.is_none() {
        return Err(ApiError::not_found(anyhow!("Bot not found")));
    }
    Ok(())
}

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
            .routes(routes!(install_bot))
            .routes(routes!(list_installed_bot))
            .routes(routes!(change_installation_status))
            .routes(routes!(uninstall_bot))
            .routes(routes!(create_bot_token))
            .routes(routes!(list_bot_tokens))
            .routes(routes!(revoke_bot_token))
            .routes(routes!(revoke_all_bot_tokens)),
    )
}

/// Install bot
#[utoipa::path(
    post,
    params(
        ("id", description = "Bots ID"),
    ),
    path = "/{id}/installations",
    responses(
        (status = 200, body = CommonResult<BotRes>, content_type = "application/json")
    ),
    tag = BOT_TAG
)]
async fn install_bot(
    state: State<MonoApiServiceState>,
    Path(id): Path<i64>,
    Json(json): Json<InstallBotReq>,
) -> Result<Json<CommonResult<BotRes>>, ApiError> {
    let bot = state
        .storage
        .bots_storage()
        .install_bot(
            id,
            json.target_type.into(),
            json.target_id,
            json.installed_by,
        )
        .await?;

    Ok(Json(CommonResult::success(Some(bot.into()))))
}

/// Get installed bot
#[utoipa::path(
    get,
    params(
        ("id", description = "Bots ID"),
    ),
    path = "/{id}/installations",
    responses(
        (status = 200, body = CommonResult<Vec<BotRes>>, content_type = "application/json")
    ),
    tag = BOT_TAG
)]
async fn list_installed_bot(
    state: State<MonoApiServiceState>,
    Path(id): Path<i64>,
) -> Result<Json<CommonResult<Vec<BotRes>>>, ApiError> {
    let models = state
        .storage
        .bots_storage()
        .get_installed_bot_by_id(id)
        .await?
        .into_iter()
        .map(|m| m.into())
        .collect();

    Ok(Json(CommonResult::success(Some(models))))
}

#[utoipa::path(
    patch,
    params(
        ("id", description = "Bot ID"),
        ("installation_id", description = "Installation ID"),
    ),
    path = "/{id}/installations/{installation_id}",
    responses(
        (status = 200, body = CommonResult<BotRes>, content_type = "application/json")
    ),
    tag = BOT_TAG
)]
async fn change_installation_status(
    state: State<MonoApiServiceState>,
    Path((id, installation_id)): Path<(i64, i64)>,
    Json(json): Json<ChangeInstallationStatus>,
) -> Result<Json<CommonResult<BotRes>>, ApiError> {
    let model = state
        .storage
        .bots_storage()
        .change_installed_bot_status(
            id,
            json.target_type.into(),
            installation_id,
            json.status.into(),
        )
        .await?;

    Ok(Json(CommonResult::success(Some(model.into()))))
}

#[utoipa::path(
    delete,
    params(
        ("id", description = "Bot ID"),
        ("installation_id", description = "Installation ID"),
    ),
    path = "/{id}/installations/{installation_id}",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = BOT_TAG
)]
async fn uninstall_bot(
    state: State<MonoApiServiceState>,
    Path((id, installation_id)): Path<(i64, i64)>,
    Json(target_type): Json<InstallationTargetType>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .storage
        .bots_storage()
        .uninstall_bot(id, target_type.into(), installation_id)
        .await?;

    Ok(Json(CommonResult::success(Some(
        "Bot uninstalled successfully".to_string(),
    ))))
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
    ensure_bot_exists(&state, bot_id).await?;

    let expires_at: Option<DateTimeWithTimeZone> = match req.expires_in {
        None => None,
        Some(seconds) => {
            if !(MIN_EXPIRES_IN_SECS..=MAX_EXPIRES_IN_SECS).contains(&seconds) {
                return Err(ApiError::bad_request(anyhow!(
                    "expires_in must be between {} and {} seconds",
                    MIN_EXPIRES_IN_SECS,
                    MAX_EXPIRES_IN_SECS
                )));
            }
            let when = Utc::now() + Duration::seconds(seconds);
            Some(DateTimeWithTimeZone::from(when))
        }
    };

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
    ensure_bot_exists(&state, bot_id).await?;

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
        (status = 200, description = "Token revoked successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "Bot not found"),
    ),
    tag = BOT_TAG
)]
async fn revoke_bot_token(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path((bot_id, token_id)): Path<(i64, i64)>,
) -> Result<Json<CommonResult<()>>, ApiError> {
    ensure_admin(&state, &user).await?;
    ensure_bot_exists(&state, bot_id).await?;

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
        (status = 200, description = "All tokens revoked successfully"),
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
    ensure_bot_exists(&state, bot_id).await?;

    state
        .storage
        .bots_storage()
        .revoke_bot_tokens_by_bot(bot_id)
        .await?;

    Ok(Json(CommonResult::success(None)))
}
