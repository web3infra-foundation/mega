use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use ceres::model::{
    create_file::CreateFileInfo,
    query::{BlobContentQuery, CodePreviewQuery},
    tree::{LatestCommitInfo, TreeBriefItem, TreeCommitItem},
};
use common::model::CommonResult;
use taurus::event::api_request::{ApiRequestEvent, ApiType};

use crate::api::mr_router;
use crate::api::user_router;
use crate::api::MonoApiServiceState;

pub fn routers() -> Router<MonoApiServiceState> {
    let router = Router::new()
        .route("/status", get(life_cycle_check))
        .route("/create-file", post(create_file))
        .route("/latest-commit", get(get_latest_commit))
        .route("/tree/commit-info", get(get_tree_commit_info))
        .route("/tree", get(get_tree_info))
        .route("/blob", get(get_blob_object));

    Router::new()
        .merge(router)
        .merge(mr_router::routers())
        .merge(user_router::routers())
}

async fn get_blob_object(
    Query(query): Query<BlobContentQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    ApiRequestEvent::notify(ApiType::Blob, &state.0.context.config);
    let res = state
        .api_handler(query.path.clone().into())
        .await
        .get_blob_as_string(query.path.into())
        .await;

    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn life_cycle_check() -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(Json("http ready"))
}

async fn create_file(
    state: State<MonoApiServiceState>,
    Json(json): Json<CreateFileInfo>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    ApiRequestEvent::notify(ApiType::CreateFile, &state.0.context.config);
    let res = state
        .api_handler(json.path.clone().into())
        .await
        .create_monorepo_file(json.clone())
        .await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn get_latest_commit(
    Query(query): Query<CodePreviewQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<LatestCommitInfo>, (StatusCode, String)> {
    ApiRequestEvent::notify(ApiType::LastestCommit, &state.0.context.config);
    let res = state
        .api_handler(query.path.clone().into())
        .await
        .get_latest_commit(query.path.into())
        .await
        .unwrap();
    Ok(Json(res))
}

async fn get_tree_info(
    Query(query): Query<CodePreviewQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<TreeBriefItem>>>, (StatusCode, String)> {
    ApiRequestEvent::notify(ApiType::TreeInfo, &state.0.context.config);
    let res = state
        .api_handler(query.path.clone().into())
        .await
        .get_tree_info(query.path.into())
        .await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn get_tree_commit_info(
    Query(query): Query<CodePreviewQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<TreeCommitItem>>>, (StatusCode, String)> {
    ApiRequestEvent::notify(ApiType::CommitInfo, &state.0.context.config);
    let res = state
        .api_handler(query.path.clone().into())
        .await
        .get_tree_commit_info(query.path.into())
        .await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}
