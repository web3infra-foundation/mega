use std::{collections::HashMap, path::PathBuf};

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use ceres::model::mr::{MRDetail, MrInfoItem};
use common::model::CommonResult;
use taurus::event::api_request::{ApiRequestEvent, ApiType};

use crate::api::error::ApiError;
use crate::api::MonoApiServiceState;

pub fn routers() -> Router<MonoApiServiceState> {
    Router::new()
        .route("/mr/list", get(get_mr_list))
        .route("/mr/:mr_id/detail", get(mr_detail))
        .route("/mr/:mr_id/merge", post(merge))
        .route("/mr/:mr_id/files", get(get_mr_files))
}

async fn merge(
    Path(mr_id): Path<i64>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    ApiRequestEvent::notify(ApiType::MergeRequest, &state.0.context.config);

    let res = state.monorepo().merge_mr(mr_id).await;
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
    Path(mr_id): Path<i64>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Option<MRDetail>>>, ApiError> {
    ApiRequestEvent::notify(ApiType::MergeDetail, &state.0.context.config);
    let res = state.monorepo().mr_detail(mr_id).await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn get_mr_files(
    Path(mr_id): Path<i64>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<PathBuf>>>, ApiError> {
    ApiRequestEvent::notify(ApiType::MergeFiles, &state.0.context.config);
    let res = state.monorepo().mr_tree_files(mr_id).await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}
