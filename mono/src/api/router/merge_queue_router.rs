use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::{Value, json};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::{MonoApiServiceState, error::ApiError};
use crate::server::http_server::MERGE_QUEUE_TAG;
use ceres::model::merge_queue::{
    AddToQueueRequest, AddToQueueResponse, QueueItem, QueueListResponse, QueueStatsResponse,
    QueueStatus, QueueStatusResponse,
};
use common::model::CommonResult;

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

/// Adds a CL to the merge queue
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
        .storage
        .merge_queue_service
        .add_to_queue(request.cl_link.clone())
        .await
    {
        Ok(position) => {
            let display_position = state
                .storage
                .merge_queue_service
                .get_display_position_by_position(position)
                .await
                .map(Some)
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        "Failed to get display position after add for {}: {}",
                        request.cl_link,
                        e
                    );
                    None
                });

            let response = AddToQueueResponse {
                success: true,
                position,
                display_position,
                message: "Added to queue".to_string(),
            };
            Ok(Json(CommonResult::success(Some(response))))
        }
        Err(e) => Ok(Json(CommonResult::failed(&e.to_string()))),
    }
}

/// Removes a CL from the merge queue
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
    state
        .storage
        .merge_queue_service
        .remove_from_queue(&cl_link)
        .await?;
    let response = json!({
        "success": true,
        "message": "Removed from queue"
    });
    Ok(Json(CommonResult::success(Some(response))))
}

/// Gets the current merge queue list
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
    let items = state.storage.merge_queue_service.get_queue_list().await?;
    let response = QueueListResponse::from(items);
    Ok(Json(CommonResult::success(Some(response))))
}

/// Gets the status of a specific CL in the queue
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
    let item_model = state
        .storage
        .merge_queue_service
        .get_cl_queue_status(&cl_link)
        .await?;

    let mut item_opt: Option<QueueItem> = item_model.map(|m| m.into());

    if let Some(ref mut item) = item_opt {
        match item.status {
            QueueStatus::Waiting | QueueStatus::Testing | QueueStatus::Merging => {
                let index_result = state
                    .storage
                    .merge_queue_service
                    .get_display_position(&item.cl_link)
                    .await;

                match index_result {
                    Ok(index) => {
                        item.display_position = index;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to get display position for {}: {}",
                            item.cl_link,
                            e
                        );
                        item.display_position = None;
                    }
                }
            }
            _ => {}
        }
    }

    let response = QueueStatusResponse {
        in_queue: item_opt.is_some(),
        item: item_opt,
    };

    Ok(Json(CommonResult::success(Some(response))))
}

/// Retries a failed queue item
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
    match state
        .storage
        .merge_queue_service
        .retry_queue_item(&cl_link)
        .await
    {
        Ok(success) => {
            let response = if success {
                json!({
                    "success": true,
                    "message": "Item retried"
                })
            } else {
                json!({
                    "success": false,
                    "message": "Item not found or cannot be retried"
                })
            };
            Ok(Json(CommonResult::success(Some(response))))
        }
        Err(e) => Ok(Json(CommonResult::failed(&e.to_string()))),
    }
}

/// Gets queue statistics
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
    let stats = state.storage.merge_queue_service.get_queue_stats().await?;
    let response = QueueStatsResponse::from(stats);
    Ok(Json(CommonResult::success(Some(response))))
}

/// Cancels all pending queue items
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
        .storage
        .merge_queue_service
        .cancel_all_pending()
        .await?;
    let response = json!({
        "success": true,
        "message": "All pending items cancelled"
    });
    Ok(Json(CommonResult::success(Some(response))))
}
