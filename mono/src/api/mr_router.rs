use std::{collections::HashMap, path::PathBuf};

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use bytes::Bytes;

use ceres::model::mr::{MRDetail, MrInfoItem};
use common::model::CommonResult;
use taurus::event::api_request::{ApiRequestEvent, ApiType};

use crate::api::error::ApiError;
use crate::api::MonoApiServiceState;

pub fn routers() -> Router<MonoApiServiceState> {
    Router::new()
        .route("/mr/list", get(get_mr_list))
        .route("/mr/:mr_link/detail", get(mr_detail))
        .route("/mr/:mr_link/merge", post(merge))
        .route("/mr/:mr_link/files", get(get_mr_files))
        .route("/mr/:mr_link/comment", post(save_comment))
}

async fn merge(
    Path(mr_link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    ApiRequestEvent::notify(ApiType::MergeRequest, &state.0.context.config);

    let res = state.monorepo().merge_mr(&mr_link).await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    ApiRequestEvent::notify(ApiType::MergeDone, &state.0.context.config);
    Ok(Json(res))
}

async fn get_mr_list(
    Query(query): Query<HashMap<String, String>>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<MrInfoItem>>>, ApiError> {
    ApiRequestEvent::notify(ApiType::MergeList, &state.0.context.config);
    let status = query.get("status").unwrap();
    let res = state.monorepo().mr_list(status).await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn mr_detail(
    Path(mr_link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Option<MRDetail>>>, ApiError> {
    ApiRequestEvent::notify(ApiType::MergeDetail, &state.0.context.config);
    let res = state.monorepo().mr_detail(&mr_link).await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn get_mr_files(
    Path(mr_link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<PathBuf>>>, ApiError> {
    ApiRequestEvent::notify(ApiType::MergeFiles, &state.0.context.config);
    let res = state.monorepo().mr_tree_files(&mr_link).await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn save_comment(
    Path(mr_link): Path<String>,
    state: State<MonoApiServiceState>,
    body: Bytes,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let json_string =
        String::from_utf8(body.to_vec()).unwrap_or_else(|_| "Invalid UTF-8".to_string());
    let res = state.monorepo().comment(&mr_link, json_string).await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}
