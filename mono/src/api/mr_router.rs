use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};

use bytes::Bytes;

use ceres::model::mr::{MRDetail, MrInfoItem};
use common::model::{CommonPage, CommonResult, RequestParams};
use saturn::ActionEnum;
use taurus::event::api_request::{ApiRequestEvent, ApiType};

use crate::api::error::ApiError;
use crate::api::model::MRStatusParams;
use crate::api::oauth::model::LoginUser;
use crate::api::util;
use crate::api::MonoApiServiceState;

pub fn routers() -> Router<MonoApiServiceState> {
    Router::new()
        .route("/mr/list", post(fetch_mr_list))
        .route("/mr/:mr_link/detail", get(mr_detail))
        .route("/mr/:mr_link/merge", post(merge))
        .route("/mr/:mr_link/files", get(get_mr_files))
        .route("/mr/:mr_link/comment", post(save_comment))
        .route("/mr/comment/:conv_id/delete", post(delete_comment))
}

async fn merge(
    user: LoginUser,
    Path(mr_link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let storage = state.context.services.mono_storage.clone();
    if let Some(model) = storage.get_open_mr_by_link(&mr_link).await.unwrap() {
        let path = model.path.clone();
        if util::check_permissions(
            &user.name,
            // "admin",
            &path,
            ActionEnum::ApproveMergeRequest,
            state.clone(),
        )
        .await
        .is_ok()
        {
            ApiRequestEvent::notify(ApiType::MergeRequest, &state.0.context.config);
            let res = state.monorepo().merge_mr(&mut model.into()).await;
            let res = match res {
                Ok(_) => CommonResult::success(None),
                Err(err) => CommonResult::failed(&err.to_string()),
            };
            ApiRequestEvent::notify(ApiType::MergeDone, &state.0.context.config);
            return Ok(Json(res));
        }
    }
    Ok(Json(CommonResult::failed("not found")))
}

async fn fetch_mr_list(
    state: State<MonoApiServiceState>,
    Json(json): Json<RequestParams<MRStatusParams>>,
) -> Result<Json<CommonResult<CommonPage<MrInfoItem>>>, ApiError> {
    ApiRequestEvent::notify(ApiType::MergeList, &state.0.context.config);
    let res = state
        .monorepo()
        .mr_list(&json.additional.status, json.pagination)
        .await;
    let res = match res {
        Ok((items, total)) => CommonResult::success(Some(CommonPage { items, total })),
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

async fn delete_comment(
    Path(conv_id): Path<i64>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.monorepo().delete_comment(conv_id).await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}
