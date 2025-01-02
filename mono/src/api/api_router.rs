use std::path::PathBuf;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use http::StatusCode;

use ceres::{
    api_service::ApiHandler,
    model::{
        create_file::CreateFileInfo,
        query::{BlobContentQuery, CodePreviewQuery},
        tree::{LatestCommitInfo, TreeBriefItem, TreeCommitItem},
    },
};
use common::{errors::ProtocolError, model::CommonResult};
use taurus::event::api_request::{ApiRequestEvent, ApiType};

use crate::api::error::ApiError;
use crate::api::issue::issue_router;
use crate::api::mr::mr_router;
use crate::api::user::user_router;
use crate::api::MonoApiServiceState;

pub fn routers() -> Router<MonoApiServiceState> {
    let router = Router::new()
        .route("/status", get(life_cycle_check))
        .route("/create-file", post(create_file))
        .route("/latest-commit", get(get_latest_commit))
        .route("/tree/commit-info", get(get_tree_commit_info))
        .route("/tree/path-can-clone", get(path_can_be_cloned))
        .route("/tree", get(get_tree_info))
        .route("/blob", get(get_blob_string))
        .route("/file/blob/{object_id}", get(get_blob_file))
        .route("/file/tree", get(get_tree_file));
    Router::new()
        .merge(router)
        .merge(mr_router::routers())
        .merge(user_router::routers())
        .merge(issue_router::routers())
}

async fn get_blob_string(
    Query(query): Query<BlobContentQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    ApiRequestEvent::notify(ApiType::Blob, &state.0.context.config);
    let res = state
        .api_handler(query.path.clone().into())
        .await?
        .get_blob_as_string(query.path.into())
        .await;

    let res = match res {
        Ok(data) => CommonResult::success(data),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn life_cycle_check() -> Result<impl IntoResponse, ApiError> {
    Ok(Json("http ready"))
}

async fn create_file(
    state: State<MonoApiServiceState>,
    Json(json): Json<CreateFileInfo>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    ApiRequestEvent::notify(ApiType::CreateFile, &state.0.context.config);
    let res = state
        .api_handler(json.path.clone().into())
        .await?
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
) -> Result<Json<LatestCommitInfo>, ApiError> {
    ApiRequestEvent::notify(ApiType::LastestCommit, &state.0.context.config);
    let res = state
        .api_handler(query.path.clone().into())
        .await?
        .get_latest_commit(query.path.into())
        .await?;
    Ok(Json(res))
}

async fn get_tree_info(
    Query(query): Query<CodePreviewQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<TreeBriefItem>>>, ApiError> {
    ApiRequestEvent::notify(ApiType::TreeInfo, &state.0.context.config);
    let res = state
        .api_handler(query.path.clone().into())
        .await?
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
) -> Result<Json<CommonResult<Vec<TreeCommitItem>>>, ProtocolError> {
    ApiRequestEvent::notify(ApiType::CommitInfo, &state.0.context.config);
    let res = state
        .api_handler(query.path.clone().into())
        .await?
        .get_tree_commit_info(query.path.into())
        .await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

pub async fn get_blob_file(
    state: State<MonoApiServiceState>,
    Path(oid): Path<String>,
) -> Result<Response, ApiError> {
    let api_handler = state.monorepo();

    let result = api_handler.get_raw_blob_by_hash(&oid).await.unwrap();
    let file_name = format!("inline; filename=\"{}\"", oid);
    match result {
        Some(model) => Ok(Response::builder()
            .header("Content-Type", "application/octet-stream")
            .header("Content-Disposition", file_name)
            .body(Body::from(model.data.unwrap()))
            .unwrap()),
        None => Ok({
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()
        }),
    }
}

pub async fn get_tree_file(
    state: State<MonoApiServiceState>,
    Query(query): Query<CodePreviewQuery>,
) -> Result<Response, ApiError> {
    let res = state
        .api_handler(query.path.clone().into())
        .await?
        .get_tree_as_data(std::path::Path::new(&query.path))
        .await;

    let file_name = format!("inline; filename=\"{}\"", "");
    match res {
        Ok(data) => Ok(Response::builder()
            .header("Content-Type", "application/octet-stream")
            .header("Content-Disposition", file_name)
            .body(Body::from(data))
            .unwrap()),
        Err(_) => Ok({
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()
        }),
    }
}

async fn path_can_be_cloned(
    Query(query): Query<BlobContentQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<bool>>, ApiError> {
    let path: PathBuf = query.path.clone().into();
    let import_dir = state.context.config.monorepo.import_dir.clone();
    let res = if path.starts_with(&import_dir) {
        state
            .context
            .services
            .git_db_storage
            .find_git_repo_exact_match(path.to_str().unwrap())
            .await
            .unwrap()
            .is_some()
    } else {
        // any path under monorepo can be cloned
        true
    };
    Ok(Json(CommonResult::success(Some(res))))
}
