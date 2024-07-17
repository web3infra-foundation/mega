use std::{collections::HashMap, path::PathBuf};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

use crate::api::ApiServiceState;
use ceres::model::{
    mr::{MRDetail, MrInfoItem},
    CommonResult,
};

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
    let res = state.monorepo().merge_mr(mr_id).await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn get_mr_list(
    Query(query): Query<HashMap<String, String>>,
    state: State<ApiServiceState>,
) -> Result<Json<CommonResult<Vec<MrInfoItem>>>, (StatusCode, String)> {
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
    let res = state.monorepo().mr_tree_files(mr_id).await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}
