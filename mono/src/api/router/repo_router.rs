use std::path::PathBuf;

use api_model::common::CommonResult;
use axum::{Json, extract::State};
use ceres::{api_service::mono::MonoServiceLogic, model::change_list::CloneRepoPayload};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::{
    MonoApiServiceState, api_doc::REPO_TAG, error::ApiError, oauth::model::LoginUser,
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
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(payload): Json<CloneRepoPayload>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let path = MonoServiceLogic::validate_github_sync_path(&payload.path)?;
    let path = PathBuf::from(path);
    state
        .monorepo()
        .sync_third_party_repo(&payload.owner, &payload.repo, path, &user.username)
        .await?;

    Ok(Json(CommonResult::success(None)))
}
