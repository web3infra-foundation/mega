use axum::{
    extract::State, 
    http::StatusCode, 
    response::IntoResponse, 
    Json
};
use uuid::Uuid;
use crate::api::{BuildRequest, AppState};
use crate::scheduler::{QueuedTask, SchedulerHandle, SchedulerConfig, TaskPriority, SchedulerError};
use crate::buck2::download_and_get_buck2_targets;

/// New task handler using the scheduler
#[utoipa::path(
    post,
    path = "/scheduler/task",
    request_body = BuildRequest,
    responses(
        (status = 200, description = "Task queued successfully", body = serde_json::Value),
        (status = 503, description = "Queue is full", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    )
)]
pub async fn scheduled_task_handler(
    State(state): State<AppState>,
    scheduler_handle: State<SchedulerHandle>,
    Json(req): Json<BuildRequest>,
) -> impl IntoResponse {
    // Download and get buck2 targets first
    let target = match download_and_get_buck2_targets(&req.buck_hash, &req.buckconfig_hash).await {
        Ok(target) => target,
        Err(e) => {
            tracing::error!("Failed to download buck2 targets: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ 
                    "error": "Failed to download buck2 targets",
                    "message": format!("{}", e) 
                })),
            ).into_response();
        }
    };

    // Create scheduler config (this should ideally be shared/cached)
    let config = SchedulerConfig::default();
    
    // Determine task priority based on request parameters
    let priority = determine_task_priority(&req);
    
    // Create a new queued task
    let task = QueuedTask::new(req, target, &config, Some(priority));
    let task_id = task.task_id;
    
    // Add task to scheduler queue
    match scheduler_handle.add_task(task).await {
        Ok(queued_task_id) => {
            tracing::info!("Task {} successfully queued", queued_task_id);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "task_id": queued_task_id.to_string(),
                    "status": "queued",
                    "message": "Task has been queued for execution"
                })),
            ).into_response()
        }
        Err(SchedulerError::QueueFull) => {
            tracing::warn!("Queue is full, rejecting task {}", task_id);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Queue is full",
                    "message": "The task queue is currently full. Please try again later."
                })),
            ).into_response()
        }
        Err(SchedulerError::TaskExists(existing_id)) => {
            tracing::warn!("Task {} already exists", existing_id);
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "Task already exists",
                    "task_id": existing_id.to_string(),
                    "message": "A task with this ID already exists"
                })),
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to queue task {}: {}", task_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to queue task",
                    "message": format!("Internal error: {}", e)
                })),
            ).into_response()
        }
    }
}

/// Get task status using scheduler
#[utoipa::path(
    get,
    path = "/scheduler/task/{id}",
    params(
        ("id" = String, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task information", body = serde_json::Value),
        (status = 404, description = "Task not found", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    )
)]
pub async fn get_scheduled_task(
    scheduler_handle: State<SchedulerHandle>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let task_uuid = match Uuid::parse_str(&task_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid task ID format",
                    "message": "Task ID must be a valid UUID"
                })),
            ).into_response();
        }
    };

    match scheduler_handle.get_task(task_uuid).await {
        Ok(Some(task)) => {
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "task_id": task.task_id.to_string(),
                    "state": format!("{:?}", task.state),
                    "priority": format!("{:?}", task.priority),
                    "retry_count": task.retry_count,
                    "max_retries": task.max_retries,
                    "created_at": task.created_at.elapsed().as_secs(),
                    "assigned_worker": task.assigned_worker,
                    "repo": task.build_request.repo,
                    "target": task.target
                })),
            ).into_response()
        }
        Ok(None) => {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Task not found",
                    "task_id": task_id,
                    "message": "No task found with the specified ID"
                })),
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get task {}: {}", task_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve task",
                    "message": format!("Internal error: {}", e)
                })),
            ).into_response()
        }
    }
}

/// Cancel a scheduled task
#[utoipa::path(
    delete,
    path = "/scheduler/task/{id}",
    params(
        ("id" = String, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task cancelled successfully", body = serde_json::Value),
        (status = 404, description = "Task not found", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    )
)]
pub async fn cancel_scheduled_task(
    scheduler_handle: State<SchedulerHandle>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let task_uuid = match Uuid::parse_str(&task_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid task ID format",
                    "message": "Task ID must be a valid UUID"
                })),
            ).into_response();
        }
    };

    match scheduler_handle.cancel_task(task_uuid).await {
        Ok(()) => {
            tracing::info!("Task {} cancelled successfully", task_id);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "task_id": task_id,
                    "status": "cancelled",
                    "message": "Task has been cancelled successfully"
                })),
            ).into_response()
        }
        Err(SchedulerError::TaskNotFound(_)) => {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Task not found",
                    "task_id": task_id,
                    "message": "No task found with the specified ID"
                })),
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to cancel task {}: {}", task_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to cancel task",
                    "message": format!("Internal error: {}", e)
                })),
            ).into_response()
        }
    }
}

/// Get scheduler statistics
#[utoipa::path(
    get,
    path = "/scheduler/stats",
    responses(
        (status = 200, description = "Scheduler statistics", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    )
)]
pub async fn get_scheduler_stats(
    scheduler_handle: State<SchedulerHandle>,
) -> impl IntoResponse {
    match scheduler_handle.get_stats().await {
        Ok(stats) => {
            (
                StatusCode::OK,
                Json(serde_json::json!(stats)),
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get scheduler stats: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to get statistics",
                    "message": format!("Internal error: {}", e)
                })),
            ).into_response()
        }
    }
}

/// Determine task priority based on build request parameters
fn determine_task_priority(req: &BuildRequest) -> TaskPriority {
    // Priority logic can be customized based on requirements
    if let Some(mr) = &req.mr {
        if mr.starts_with("hotfix") || mr.contains("urgent") {
            return TaskPriority::Critical;
        }
        if mr.starts_with("feature") {
            return TaskPriority::High;
        }
    }
    
    // Check if this is a quick build (fewer args typically means simpler build)
    if let Some(args) = &req.args {
        if args.len() <= 2 {
            return TaskPriority::High;
        }
    }
    
    TaskPriority::Normal
}

/// Helper function to create scheduler routes
pub fn scheduler_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/scheduler/task", axum::routing::post(scheduled_task_handler))
        .route("/scheduler/task/:id", axum::routing::get(get_scheduled_task))
        .route("/scheduler/task/:id", axum::routing::delete(cancel_scheduled_task))
        .route("/scheduler/stats", axum::routing::get(get_scheduler_stats))
}
