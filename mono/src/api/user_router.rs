use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

use common::model::CommonResult;

use crate::api::oauth::model::GitHubUserJson;
use crate::api::MonoApiServiceState;

pub fn routers() -> Router<MonoApiServiceState> {
    Router::new()
        .route("/user", get(user))
        .route("/user/add_key", post(add_key))
}

async fn user(
    user: GitHubUserJson,
    _: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<GitHubUserJson>>, (StatusCode, String)> {
    Ok(Json(CommonResult::success(Some(user))))
}

async fn add_key(
    user: GitHubUserJson,
    _: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    tracing::info!("user:{:?}", user);
    todo!()
}
