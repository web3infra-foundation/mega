use crate::api::gpg::model::{GpgKey, NewGpgRequest, RemoveGpgRequest};
use crate::api::oauth::model::LoginUser;
use crate::{
    api::{MonoApiServiceState, error::ApiError},
    server::http_server::GPG_TAG,
};
use axum::{Json, extract::State};
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
    delete,
    path = "/remove",
    request_body = RemoveGpgRequest,
    responses(
        (status = 200, body = CommonResult<String>, content_type="application/json")
    ),
    tag = GPG_TAG
)]
async fn remove_gpg(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(req): Json<RemoveGpgRequest>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    // let uid = "exampleid".to_string();
    let uid = user.campsite_user_id.clone();
    state.gpg_stg().remove_gpg_key(uid, req.key_id).await?;
    Ok(Json(CommonResult::success(None)))
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
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(req): Json<NewGpgRequest>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    // let uid = "exampleid".to_string();
    let uid = user.campsite_user_id.clone();
    println!("Adding GPG key for user: {}", req.gpg_content.clone());
    state.gpg_stg().add_gpg_key(uid, req.gpg_content).await?;

    Ok(Json(CommonResult::success(None)))
}
#[utoipa::path(
    get,
    path = "/list",
    responses(
        (status = 200, body = CommonResult<Vec<GpgKey>>, content_type="application/json")
    ),
    tag = GPG_TAG
)]
async fn list_gpg(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<GpgKey>>>, ApiError> {
    // let uid = "exampleid".to_string();
    let uid = user.campsite_user_id;
    let raw_keys = state.gpg_stg().list_user_gpg(uid.clone()).await;

    let res: Vec<GpgKey> = raw_keys
        .into_iter()
        .flatten()
        .map(|k: Model| GpgKey {
            user_id: uid.clone(),
            key_id: k.key_id,
            fingerprint: k.fingerprint,
            created_at: k.created_at.and_utc(),
            expires_at: k.expires_at.map(|dt| dt.and_utc()),
        })
        .collect();

    Ok(Json(CommonResult::success(Some(res))))
}
