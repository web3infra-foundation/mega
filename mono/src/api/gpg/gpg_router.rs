use axum::{extract::State, Json};
use common::model::CommonResult;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{api::{error::ApiError, MonoApiServiceState}, server::http_server::GPG_TAG};
use crate::api::gpg::model::NewGpgRequest;

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/gpg",
        OpenApiRouter::new()
        .routes(routes!(add_gpg)),
    )
}

#[utoipa::path(
    post,
    path = "/add",
    request_body = NewGpgRequest,
    responses(
        (status = 200, body = CommonResult<String>, content_type="application/json")
    ),
    tag = GPG_TAG
)]
async fn add_gpg(
    state: State<MonoApiServiceState>,
    Json(req): Json<NewGpgRequest>
) -> Result<Json<CommonResult<String>>, ApiError> {
    let _ = state
        .gpg_stg()
        .add_gpg_key(
            req.user_id,
            req.gpg_content,
            req.expires_days)
        .await;
    Ok(Json(CommonResult::success(None)))
}


