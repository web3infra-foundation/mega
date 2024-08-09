use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use mq::event::api_request::{ApiRequestEvent, ApiType};
use ceres::model::{
    create_file::CreateFileInfo,
    publish_path::PublishPathInfo,
    query::{BlobContentQuery, CodePreviewQuery},
    tree::{LatestCommitInfo, TreeBriefItem, TreeCommitItem},
};
use common::model::CommonResult;

use crate::api::mr_router;
use crate::api::ApiServiceState;

use super::ztm_router;

pub fn routers() -> Router<ApiServiceState> {
    let router = Router::new()
        .route("/status", get(life_cycle_check))
        .route("/create-file", post(create_file))
        .route("/latest-commit", get(get_latest_commit))
        .route("/tree/commit-info", get(get_tree_commit_info))
        .route("/tree", get(get_tree_info))
        .route("/blob", get(get_blob_object))
        .route("/publish", post(publish_path_to_repo));

    Router::new()
        .merge(router)
        .merge(mr_router::routers())
        .merge(ztm_router::routers())
}

async fn get_blob_object(
    Query(query): Query<BlobContentQuery>,
    state: State<ApiServiceState>,
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

// async fn get_origin_object(
//     Query(query): Query<HashMap<String, String>>,
//     state: State<ApiServiceState>,
// ) -> Result<impl IntoResponse, (StatusCode, String)> {
//     let object_id = query.get("object_id").unwrap();
//     let repo_path = query.get("repo_path").expect("repo_path is required");
//     state
//         .object_service
//         .get_objects_data(object_id, repo_path)
//         .await
// }

async fn life_cycle_check() -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(Json("http ready"))
}

// async fn get_count_nums(
//     Query(query): Query<HashMap<String, String>>,
//     state: State<ApiServiceState>,
// ) -> Result<Json<GitTypeCounter>, (StatusCode, String)> {
//     let repo_path = query.get("repo_path").unwrap();
//     state.object_service.count_object_num(repo_path).await
// }

async fn create_file(
    state: State<ApiServiceState>,
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
    state: State<ApiServiceState>,
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
    state: State<ApiServiceState>,
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
    state: State<ApiServiceState>,
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

async fn publish_path_to_repo(
    state: State<ApiServiceState>,
    Json(json): Json<PublishPathInfo>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    ApiRequestEvent::notify(ApiType::Publish, &state.0.context.config);
    let res = state
        .api_handler(json.path.clone().into())
        .await
        .publish_path(json)
        .await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}
