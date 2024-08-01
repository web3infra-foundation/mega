use std::{collections::HashMap, path::PathBuf};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

use ceres::model::mr::{MRDetail, MrInfoItem};
use common::model::CommonResult;

use crate::{api::ApiServiceState, mq::event::{ApiRequestEvent, ApiType}};

pub fn routers() -> Router<ApiServiceState> {
    Router::new()
        .route("/mr/list", get(get_mr_list))
        .route("/mr/:mr_id/detail", get(mr_detail))
        .route("/mr/:mr_id/merge", post(merge))
        .route("/mr/:mr_id/files", get(get_mr_files))
}

async fn merge(
    Path(mr_id): Path<i64>,
    state: State<ApiServiceState>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    ApiRequestEvent::notice(ApiType::MergeRequest, &state);

    let res = state.monorepo().merge_mr(mr_id).await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    ApiRequestEvent::notice(ApiType::MergeDone, &state);
    Ok(Json(res))
}

async fn get_mr_list(
    Query(query): Query<HashMap<String, String>>,
    state: State<ApiServiceState>,
) -> Result<Json<CommonResult<Vec<MrInfoItem>>>, (StatusCode, String)> {
    ApiRequestEvent::notice(ApiType::MergeList, &state);
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
    state: State<ApiServiceState>,
) -> Result<Json<CommonResult<Option<MRDetail>>>, (StatusCode, String)> {
    ApiRequestEvent::notice(ApiType::MergeDetail, &state);
    let res = state.monorepo().mr_detail(mr_id).await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn get_mr_files(
    Path(mr_id): Path<i64>,
    state: State<ApiServiceState>,
) -> Result<Json<CommonResult<Vec<PathBuf>>>, (StatusCode, String)> {
    ApiRequestEvent::notice(ApiType::MergeFiles, &state);
    let res = state.monorepo().mr_tree_files(mr_id).await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}
