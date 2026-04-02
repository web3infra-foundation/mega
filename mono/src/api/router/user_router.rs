use api_model::common::CommonResult;
use axum::{
    Json,
    extract::{Path, State},
    routing::get,
};
use ceres::model::{
    notification::{
        NotificationEventTypeInfo, UpdateUserNotificationConfig, UserNotificationConfig,
        UserNotificationPreferenceItem,
    },
    user::{
        AddSSHKey, ClaContentRes, ClaSignStatusRes, ListSSHKey, ListToken, UpdateClaContentPayload,
    },
};
use common::errors::MegaError;
use russh::keys::{HashAlg, parse_public_key_base64};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::{
    MonoApiServiceState, api_doc::USER_TAG, error::ApiError, oauth::model::LoginUser,
};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/user",
        OpenApiRouter::new()
            .route("/", get(user))
            .routes(routes!(list_key))
            .routes(routes!(add_key))
            .routes(routes!(remove_key))
            .routes(routes!(generate_token))
            .routes(routes!(list_token))
            .routes(routes!(remove_token))
            .routes(routes!(list_notification_types))
            .routes(routes!(get_notification_config))
            .routes(routes!(update_notification_config))
            .routes(routes!(get_cla_sign_status))
            .routes(routes!(change_sign_status))
            .routes(routes!(get_cla_content))
            .routes(routes!(update_cla_content)),
    )
}

async fn user(
    user: LoginUser,
    _: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<LoginUser>>, ApiError> {
    Ok(Json(CommonResult::success(Some(user))))
}

/// Add SSH Key
#[utoipa::path(
    post,
    path = "/ssh",
    request_body = AddSSHKey,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn add_key(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(json): Json<AddSSHKey>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let ssh_parts: Vec<&str> = json.ssh_key.split_whitespace().collect();
    let key = parse_public_key_base64(
        ssh_parts
            .get(1)
            .ok_or_else(|| MegaError::Other("Invalid key format".to_string()))?,
    )?;
    let title = if json.title.is_empty() {
        ssh_parts
            .get(2)
            .ok_or_else(|| MegaError::Other("Invalid key format".to_string()))?
            .to_string()
    } else {
        json.title
    };
    state
        .user_stg()
        .save_ssh_key(
            user.username,
            &title,
            &json.ssh_key,
            &key.fingerprint(HashAlg::Sha256).to_string(),
        )
        .await?;
    Ok(Json(CommonResult::success(None)))
}

/// Delete SSH Key
#[utoipa::path(
    delete,
        params(
        ("key_id", description = "A numeric ID representing a SSH"),
    ),
    path = "/ssh/{key_id}",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn remove_key(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Path(key_id): Path<i64>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .user_stg()
        .delete_ssh_key(user.username, key_id)
        .await?;
    Ok(Json(CommonResult::success(None)))
}

/// Get User's SSH key list
#[utoipa::path(
    get,
    path = "/ssh/list",
    responses(
        (status = 200, body = CommonResult<Vec<ListSSHKey>>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn list_key(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<ListSSHKey>>>, ApiError> {
    let res = state.user_stg().list_user_ssh(user.username).await?;
    Ok(Json(CommonResult::success(Some(
        res.into_iter().map(|x| x.into()).collect(),
    ))))
}

/// Generate Token For http push
#[utoipa::path(
    post,
    path = "/token/generate",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn generate_token(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.user_stg().generate_token(user.username).await?;
    Ok(Json(CommonResult::success(Some(res))))
}

/// Delete User's http push token
#[utoipa::path(
    delete,
        params(
        ("key_id", description = "A numeric ID representing a User Token"),
    ),
    path = "/token/{key_id}",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn remove_token(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Path(key_id): Path<i64>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state.user_stg().delete_token(user.username, key_id).await?;
    Ok(Json(CommonResult::success(None)))
}

/// Get User's push token list
#[utoipa::path(
    get,
    path = "/token/list",
    responses(
        (status = 200, body = CommonResult<Vec<ListToken>>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn list_token(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<ListToken>>>, ApiError> {
    let data = state.user_stg().list_token(user.username).await?;
    let res = data.into_iter().map(|x| x.into()).collect();
    Ok(Json(CommonResult::success(Some(res))))
}

/// List supported notification event types
#[utoipa::path(
    get,
    path = "/notification/types",
    responses((status = 200, body = CommonResult<Vec<NotificationEventTypeInfo>>)),
    tag = USER_TAG
)]
async fn list_notification_types(
    _user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<NotificationEventTypeInfo>>>, ApiError> {
    let types = state
        .notification_stg()
        .list_event_types()
        .await?
        .into_iter()
        .map(|t| NotificationEventTypeInfo {
            code: t.code,
            category: t.category,
            description: t.description,
            system_required: t.system_required,
            default_enabled: t.default_enabled,
        })
        .collect();

    Ok(Json(CommonResult::success(Some(types))))
}

/// Get current user's notification config
#[utoipa::path(
    get,
    path = "/notification/config",
    responses((status = 200, body = CommonResult<UserNotificationConfig>)),
    tag = USER_TAG
)]
async fn get_notification_config(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<UserNotificationConfig>>, ApiError> {
    state
        .notification_stg()
        .upsert_user_settings(&user.username, &user.email)
        .await?;

    let settings = state
        .notification_stg()
        .get_user_settings(&user.username)
        .await?
        .ok_or_else(|| MegaError::Other("user settings missing".to_string()))?;

    let prefs = state
        .notification_stg()
        .list_user_preferences(&user.username)
        .await?
        .into_iter()
        .map(|p| UserNotificationPreferenceItem {
            event_type_code: p.event_type_code,
            enabled: p.enabled,
        })
        .collect();

    Ok(Json(CommonResult::success(Some(UserNotificationConfig {
        enabled: settings.enabled,
        delivery_mode: settings.delivery_mode,
        email: settings.email,
        preferences: prefs,
    }))))
}

/// Update current user's notification config
#[utoipa::path(
    put,
    path = "/notification/config",
    request_body = UpdateUserNotificationConfig,
    responses((status = 200, body = CommonResult<String>)),
    tag = USER_TAG
)]
async fn update_notification_config(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(payload): Json<UpdateUserNotificationConfig>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .notification_stg()
        .upsert_user_settings(&user.username, &user.email)
        .await?;

    if let Some(enabled) = payload.enabled {
        state
            .notification_stg()
            .set_global_enabled(&user.username, enabled)
            .await?;
    }
    if let Some(mode) = payload.delivery_mode {
        state
            .notification_stg()
            .set_delivery_mode(&user.username, &mode)
            .await?;
    }
    if let Some(items) = payload.preferences {
        for item in items {
            state
                .notification_stg()
                .set_user_preference(&user.username, &item.event_type_code, item.enabled)
                .await?;
        }
    }

    Ok(Json(CommonResult::success(None)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_router_contains_notification_routes() {
        let _router = routers();
    }
}
/// Get current user's CLA sign status
#[utoipa::path(
    get,
    path = "/cla/status",
    responses(
        (status = 200, body = CommonResult<ClaSignStatusRes>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn get_cla_sign_status(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<ClaSignStatusRes>>, ApiError> {
    let (cla_signed, cla_signed_at) = state
        .monorepo()
        .get_or_init_cla_sign_status(&user.username)
        .await?;

    let res = ClaSignStatusRes {
        username: user.username,
        cla_signed,
        cla_signed_at: cla_signed_at.map(|dt| dt.and_utc().timestamp()),
    };
    Ok(Json(CommonResult::success(Some(res))))
}

/// Change CLA sign status for current user
#[utoipa::path(
    post,
    path = "/cla/change-sign-status",
    responses(
        (status = 200, body = CommonResult<ClaSignStatusRes>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn change_sign_status(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<ClaSignStatusRes>>, ApiError> {
    let (cla_signed, cla_signed_at) = state
        .monorepo()
        .change_cla_sign_status(&user.username)
        .await?;

    let res = ClaSignStatusRes {
        username: user.username,
        cla_signed,
        cla_signed_at: cla_signed_at.map(|dt| dt.and_utc().timestamp()),
    };
    Ok(Json(CommonResult::success(Some(res))))
}

/// Get latest CLA text content
#[utoipa::path(
    get,
    path = "/cla/content",
    responses(
        (status = 200, body = CommonResult<ClaContentRes>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn get_cla_content(
    _user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<ClaContentRes>>, ApiError> {
    let content = state.monorepo().get_cla_content().await?;
    Ok(Json(CommonResult::success(Some(ClaContentRes { content }))))
}

/// Update latest CLA text content
#[utoipa::path(
    post,
    path = "/cla/content",
    request_body = UpdateClaContentPayload,
    responses(
        (status = 200, body = CommonResult<ClaContentRes>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn update_cla_content(
    _user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(payload): Json<UpdateClaContentPayload>,
) -> Result<Json<CommonResult<ClaContentRes>>, ApiError> {
    state
        .monorepo()
        .update_cla_content(&payload.content)
        .await?;
    Ok(Json(CommonResult::success(Some(ClaContentRes {
        content: payload.content,
    }))))
}
