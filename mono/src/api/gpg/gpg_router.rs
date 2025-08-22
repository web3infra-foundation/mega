use crate::api::gpg::model::{GpgKey, NewGpgRequest, RemoveGpgRequest};
use crate::{
    api::{error::ApiError, MonoApiServiceState},
    server::http_server::GPG_TAG,
};
use axum::{extract::State, Json};
use callisto::gpg_key::Model;
use common::model::CommonResult;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/gpg",
        OpenApiRouter::new()
            .routes(routes!(add_gpg))
            .routes(routes!(remove_gpg))
            .routes(routes!(list_gpg)),
    )
}

#[utoipa::path(
    post,
    path = "/add",
    request_body = RemoveGpgRequest,
    responses(
        (status = 200, body = CommonResult<String>, content_type="application/json")
    ),
    tag = GPG_TAG
)]
async fn add_gpg(
    state: State<MonoApiServiceState>,
    Json(req): Json<RemoveGpgRequest>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let _ = state
        .gpg_stg()
        .remove_gpg_key(req.user_id, req.key_id)
        .await;
    Ok(Json(CommonResult::success(None)))
}

#[utoipa::path(
    delete,
    path = "/remove",
    request_body = NewGpgRequest,
    responses(
        (status = 200, body = CommonResult<String>, content_type="application/json")
    ),
    tag = GPG_TAG
)]
async fn remove_gpg(
    state: State<MonoApiServiceState>,
    Json(req): Json<NewGpgRequest>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let _ = state
        .gpg_stg()
        .add_gpg_key(req.user_id, req.gpg_content, req.expires_days)
        .await;
    Ok(Json(CommonResult::success(None)))
}
#[utoipa::path(
    get,
    params(
        ("id" = i64, description = "The user ID"),
    ),
    path = "/list/{id}",
    responses(
        (status = 200, body = CommonResult<Vec<GpgKey>>, content_type="application/json")
    ),
    tag = GPG_TAG
)]
async fn list_gpg(
    state: State<MonoApiServiceState>,
    Json(req): Json<NewGpgRequest>,
) -> Result<Json<CommonResult<Vec<GpgKey>>>, ApiError> {
    let raw_keys = state.gpg_stg().list_user_gpg(req.user_id).await;

    let res: Vec<GpgKey> = raw_keys
        .into_iter()
        .flatten()
        .map(|k: Model| GpgKey {
            user_id: k.user_id,
            key_id: k.key_id,
            fingerprint: k.fingerprint,
            created_at: k.created_at.and_utc(),
            expires_at: k.expires_at.map(|dt| dt.and_utc()),
        })
        .collect();

    Ok(Json(CommonResult::success(Some(res))))
}
