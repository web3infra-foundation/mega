use api_model::common::CommonResult;
use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::bots::{BotRes, ChangeInstallationStatus, InstallBotReq, InstallationTargetType};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{MonoApiServiceState, error::ApiError},
    server::http_server::BOTS_TAG,
};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/bots",
        OpenApiRouter::new()
            .routes(routes!(install_bot))
            .routes(routes!(list_installed_bot))
            .routes(routes!(change_installation_status))
            .routes(routes!(uninstall_bot)),
    )
}

/// Install bot
#[utoipa::path(
    post,
    params(
        ("id", description = "Bots ID"),
    ),
    path = "/{id}/installations ",
    responses(
        (status = 200, body = CommonResult<BotRes>, content_type = "application/json")
    ),
    tag = BOTS_TAG
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
    path = "/{id}/installations ",
    responses(
        (status = 200, body = CommonResult<Vec<BotRes>>, content_type = "application/json")
    ),
    tag = BOTS_TAG
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
    tag = BOTS_TAG
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
    tag = BOTS_TAG
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
