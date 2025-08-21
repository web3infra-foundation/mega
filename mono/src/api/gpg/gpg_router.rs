use axum::{extract::State, Json};
use common::model::CommonResult;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{api::{error::ApiError, MonoApiServiceState}, server::http_server::GPG_TAG};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/gpg",
        OpenApiRouter::new()
        .routes(routes!(gpg)),
    )
}

#[utoipa::path(
    get,
    path = "/test",
    responses(
        (status = 200, body = CommonResult<String>, content_type="application/json")
    ),
    tag = GPG_TAG
)]
async fn gpg(
    state: State<MonoApiServiceState>
) -> Result<Json<CommonResult<String>>, ApiError> {
    state.gpg_stg().save_gpg_key().await?;
    Ok(Json(CommonResult::success(Some("success".to_string()))))
} 