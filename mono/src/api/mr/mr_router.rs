use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};

use bytes::Bytes;

use callisto::db_enums::{ConvType, MergeStatus};
use ceres::protocol::mr::MergeRequest;
use common::model::{CommonPage, CommonResult, PageParams};
use saturn::ActionEnum;
use taurus::event::api_request::{ApiRequestEvent, ApiType};

use crate::api::error::ApiError;
use crate::api::mr::{MRDetail, MRStatusParams, MrInfoItem};
use crate::api::oauth::model::LoginUser;
use crate::api::util;
use crate::api::MonoApiServiceState;

pub fn routers() -> Router<MonoApiServiceState> {
    Router::new()
        .route("/mr/list", post(fetch_mr_list))
        .route("/mr/:link/detail", get(mr_detail))
        .route("/mr/:link/merge", post(merge))
        .route("/mr/:link/close", post(close_mr))
        .route("/mr/:link/reopen", post(reopen_mr))
        .route("/mr/:link/files", get(get_mr_files))
        .route("/mr/:link/files-diff", get(get_mr_files_diff))
        .route("/mr/:link/comment", post(save_comment))
        .route("/mr/comment/:conv_id/delete", post(delete_comment))
}

async fn reopen_mr(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    if let Some(model) = state.mr_stg().get_mr(&link).await.unwrap() {
        if model.status == MergeStatus::Closed {
            util::check_permissions(
                &user.name,
                &model.path,
                ActionEnum::EditMergeRequest,
                state.clone(),
            )
            .await
            .unwrap();
            let mut mr: MergeRequest = model.into();
            mr.status = MergeStatus::Open;
            let res = match state
                .mr_stg()
                .reopen_mr(mr.into(), user.user_id, &user.name)
                .await
            {
                Ok(_) => CommonResult::success(None),
                Err(err) => CommonResult::failed(&err.to_string()),
            };
            return Ok(Json(res));
        }
    }
    Ok(Json(CommonResult::failed("not found")))
}

async fn close_mr(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    if let Some(model) = state.mr_stg().get_mr(&link).await.unwrap() {
        if model.status == MergeStatus::Open {
            util::check_permissions(
                &user.name,
                &model.path,
                ActionEnum::EditMergeRequest,
                state.clone(),
            )
            .await
            .unwrap();
            let mut mr: MergeRequest = model.into();
            mr.status = MergeStatus::Closed;
            let res = match state
                .mr_stg()
                .close_mr(mr.into(), user.user_id, &user.name)
                .await
            {
                Ok(_) => CommonResult::success(None),
                Err(err) => CommonResult::failed(&err.to_string()),
            };
            return Ok(Json(res));
        }
    }
    Ok(Json(CommonResult::failed("not found")))
}

async fn merge(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    if let Some(model) = state.mr_stg().get_mr(&link).await.unwrap() {
        if model.status == MergeStatus::Open {
            let path = model.path.clone();
            util::check_permissions(
                &user.name,
                &path,
                ActionEnum::ApproveMergeRequest,
                state.clone(),
            )
            .await
            .unwrap();
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
    Json(json): Json<PageParams<MRStatusParams>>,
) -> Result<Json<CommonResult<CommonPage<MrInfoItem>>>, ApiError> {
    ApiRequestEvent::notify(ApiType::MergeList, &state.0.context.config);
    let status = json.additional.status;
    let status = if status == "open" {
        vec![MergeStatus::Open]
    } else if status == "closed" {
        vec![MergeStatus::Closed, MergeStatus::Merged]
    } else {
        vec![MergeStatus::Open, MergeStatus::Closed, MergeStatus::Merged]
    };
    let res = match state
        .mr_stg()
        .get_mr_by_status(status, json.pagination.page, json.pagination.per_page)
        .await
    {
        Ok((items, total)) => CommonResult::success(Some(CommonPage {
            items: items.into_iter().map(|m| m.into()).collect(),
            total,
        })),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn mr_detail(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<MRDetail>>, ApiError> {
    ApiRequestEvent::notify(ApiType::MergeDetail, &state.0.context.config);
    let res = match state.mr_stg().get_mr(&link).await {
        Ok(data) => {
            if let Some(model) = data {
                let mut detail: MRDetail = model.into();
                let conversions = state.mr_stg().get_mr_conversations(&link).await.unwrap();
                detail.conversions = conversions.into_iter().map(|x| x.into()).collect();
                CommonResult::success(Some(detail))
            } else {
                CommonResult::success(None)
            }
        }
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn get_mr_files(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<PathBuf>>>, ApiError> {
    ApiRequestEvent::notify(ApiType::MergeFiles, &state.0.context.config);
    let res = state.monorepo().mr_tree_files(&link).await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn get_mr_files_diff(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.monorepo().content_diff(&link).await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn save_comment(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    body: Bytes,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let json_string =
        String::from_utf8(body.to_vec()).unwrap_or_else(|_| "Invalid UTF-8".to_string());

    let res = if let Some(model) = state.mr_stg().get_mr(&link).await.unwrap() {
        state
            .mr_stg()
            .add_mr_conversation(
                &model.link,
                user.user_id,
                ConvType::Comment,
                Some(json_string),
            )
            .await
            .unwrap();
        CommonResult::success(None)
    } else {
        CommonResult::failed("Invalid link")
    };
    Ok(Json(res))
}

async fn delete_comment(
    Path(conv_id): Path<i64>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = match state.mr_stg().remove_mr_conversation(conv_id).await {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}
