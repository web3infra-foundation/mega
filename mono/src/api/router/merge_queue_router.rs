use api_model::common::CommonResult;
use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::merge_queue::{
    AddToQueueRequest, AddToQueueResponse, QueueListResponse, QueueStatsResponse,
    QueueStatusResponse,
};
use serde_json::{Value, json};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::{MonoApiServiceState, api_doc::MERGE_QUEUE_TAG, error::ApiError};

/// Creates the merge queue router with all endpoints
pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/merge-queue",
        OpenApiRouter::new()
            .routes(routes!(add_to_queue))
            .routes(routes!(remove_from_queue))
            .routes(routes!(get_queue_list))
            .routes(routes!(get_cl_queue_status))
            .routes(routes!(retry_queue_item))
            .routes(routes!(get_queue_stats))
            .routes(routes!(cancel_all_pending)),
    )
}

#[utoipa::path(
    post,
    path = "/add",
    request_body = AddToQueueRequest,
    responses(
        (status = 200, body = CommonResult<AddToQueueResponse>, content_type = "application/json")
    ),
    tag = MERGE_QUEUE_TAG
)]
async fn add_to_queue(
    state: State<MonoApiServiceState>,
    Json(request): Json<AddToQueueRequest>,
) -> Result<Json<CommonResult<AddToQueueResponse>>, ApiError> {
    match state
        .services()
        .cl()
        .add_to_merge_queue_response(request.cl_link)
        .await
    {
        Ok(response) => Ok(Json(CommonResult::success(Some(response)))),
        Err(e) => Ok(Json(CommonResult::failed(&e.to_string()))),
    }
}

#[utoipa::path(
    delete,
    path = "/remove/{cl_link}",
    params(
        ("cl_link" = String, Path, description = "CL link to remove")
    ),
    responses(
        (status = 200, body = CommonResult<Value>, content_type = "application/json")
    ),
    tag = MERGE_QUEUE_TAG
)]
async fn remove_from_queue(
    state: State<MonoApiServiceState>,
    Path(cl_link): Path<String>,
) -> Result<Json<CommonResult<Value>>, ApiError> {
    match state
        .services()
        .cl()
        .remove_from_merge_queue(&cl_link)
        .await
    {
        Ok(true) => Ok(Json(CommonResult::success(Some(json!({
            "success": true,
            "message": "Removed from queue"
        }))))),
        Ok(false) => Ok(Json(CommonResult::failed("CL not found in queue"))),
        Err(e) => Ok(Json(CommonResult::failed(&e.to_string()))),
    }
}

#[utoipa::path(
    get,
    path = "/list",
    responses(
        (status = 200, body = CommonResult<QueueListResponse>, content_type = "application/json")
    ),
    tag = MERGE_QUEUE_TAG
)]
async fn get_queue_list(
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<QueueListResponse>>, ApiError> {
    let response = state.services().cl().get_merge_queue_list().await?;
    Ok(Json(CommonResult::success(Some(response))))
}

#[utoipa::path(
    get,
    path = "/status/{cl_link}",
    params(
        ("cl_link" = String, Path, description = "CL link to check status")
    ),
    responses(
        (status = 200, body = CommonResult<QueueStatusResponse>, content_type = "application/json")
    ),
    tag = MERGE_QUEUE_TAG
)]
async fn get_cl_queue_status(
    state: State<MonoApiServiceState>,
    Path(cl_link): Path<String>,
) -> Result<Json<CommonResult<QueueStatusResponse>>, ApiError> {
    let response = state.services().cl().get_cl_queue_status(&cl_link).await?;
    Ok(Json(CommonResult::success(Some(response))))
}

#[utoipa::path(
    post,
    path = "/retry/{cl_link}",
    params(
        ("cl_link" = String, Path, description = "The cl_link to retry")
    ),
    responses(
        (status = 200, description = "Successfully retried item", body = CommonResult<Value>),
        (status = 404, description = "Item not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = MERGE_QUEUE_TAG
)]
async fn retry_queue_item(
    state: State<MonoApiServiceState>,
    Path(cl_link): Path<String>,
) -> Result<Json<CommonResult<Value>>, ApiError> {
    match state.services().cl().retry_merge_queue_item(&cl_link).await {
        Ok(true) => Ok(Json(CommonResult::success(Some(json!({
            "success": true,
            "message": "Item retried"
        }))))),
        Ok(false) => Ok(Json(CommonResult::success(Some(json!({
            "success": false,
            "message": "Item not found or cannot be retried"
        }))))),
        Err(e) => Ok(Json(CommonResult::failed(&e.to_string()))),
    }
}

#[utoipa::path(
    get,
    path = "/stats",
    responses(
        (status = 200, body = CommonResult<QueueStatsResponse>, content_type = "application/json")
    ),
    tag = MERGE_QUEUE_TAG
)]
async fn get_queue_stats(
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<QueueStatsResponse>>, ApiError> {
    let response = state.services().cl().get_merge_queue_stats().await?;
    Ok(Json(CommonResult::success(Some(response))))
}

#[utoipa::path(
    post,
    path = "/cancel-all",
    responses(
        (status = 200, body = CommonResult<Value>, content_type = "application/json")
    ),
    tag = MERGE_QUEUE_TAG
)]
async fn cancel_all_pending(
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Value>>, ApiError> {
    state
        .services()
        .cl()
        .cancel_all_pending_merge_queue()
        .await?;
    Ok(Json(CommonResult::success(Some(json!({
        "success": true,
        "message": "All pending items cancelled"
    })))))
}
