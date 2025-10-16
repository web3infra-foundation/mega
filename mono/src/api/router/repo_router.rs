use std::path::PathBuf;

use axum::{Json, extract::State};
use ceres::model::change_list::CloneRepoPayload;
use common::model::CommonResult;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{MonoApiServiceState, error::ApiError},
    server::http_server::REPO_TAG,
};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/repo",
        OpenApiRouter::new().routes(routes!(clone_third_party_repo)),
    )
}

// Clone a Github Repo
#[utoipa::path(
    post,
    path = "/clone",
    request_body (
        content = CloneRepoPayload,
    ),
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = REPO_TAG
)]
async fn clone_third_party_repo(
    state: State<MonoApiServiceState>,
    Json(payload): Json<CloneRepoPayload>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let path = PathBuf::from(payload.path);
    state
        .monorepo()
        .sync_third_party_repo(&payload.owner, &payload.repo, path)
        .await?;

    Ok(Json(CommonResult::success(None)))
}
