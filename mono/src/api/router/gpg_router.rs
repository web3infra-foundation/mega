use api_model::common::CommonResult;
use axum::{Json, extract::State};
use ceres::model::gpg::{GpgKey, NewGpgRequest, RemoveGpgRequest};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::{MonoApiServiceState, api_doc::GPG_TAG, error::ApiError, oauth::model::LoginUser};
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
    state.monorepo().remove_gpg_key(uid, req.key_id).await?;
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
    state.monorepo().add_gpg_key(uid, req.gpg_content).await?;

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
    let res = state.monorepo().list_user_gpg_keys(uid).await?;

    Ok(Json(CommonResult::success(Some(res))))
}
