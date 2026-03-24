use std::net::SocketAddr;

use anyhow::Result;
use api_model::{
    buck2::{
        api::{RetryBuildRequest, TaskBuildRequest},
        types::{
            LogErrorResponse, LogLinesResponse, TargetLogLinesResponse, TargetLogQuery,
            TargetStatusResponse, TaskHistoryQuery,
        },
    },
    common::{CommonPage, PageParams},
};
use axum::{
    Json, Router,
    extract::{ConnectInfo, Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{any, get, post},
};

use crate::{
    app_state::AppState,
    model::dto::{
        BuildEventDTO, BuildEventState, BuildTargetDTO, MessageResponse, OrionClientInfo,
        OrionClientQuery, OrionClientStatus, OrionTaskDTO, TargetSummaryDTO, TaskInfoDTO,
    },
    scheduler::TaskQueueStats,
    service::{api_v2_service, ws_service},
};

/// Creates and configures all API routes
pub fn routers() -> Router<AppState> {
    Router::new()
        .merge(system_routes())
        .merge(task_routes())
        .merge(build_routes())
        .merge(worker_routes())
        .merge(target_status_routes())
}

fn system_routes() -> Router<AppState> {
    Router::new()
        .route("/ws", any(ws_handler))
        .route("/v2/health", get(health_check_handler))
        .route("/queue-stats", get(queue_stats_handler))
}

fn task_routes() -> Router<AppState> {
    Router::new()
        .route("/task", post(task_handler))
        .route("/v2/task-handler", get(task_handler_v2))
        .route("/task-build-list/{id}", get(task_build_list_handler))
        .route("/task-output/{id}", get(task_output_handler))
        .route("/task-history-output", get(task_history_output_handler))
        .route("/tasks/{cl}", get(tasks_handler))
        .route("/tasks/{task_id}/targets", get(task_targets_handler))
        .route(
            "/tasks/{task_id}/targets/summary",
            get(task_targets_summary_handler),
        )
        .route("/v2/task-retry/{id}", post(task_retry_handler))
        .route("/v2/task/{cl}", get(task_get_handler))
}

fn build_routes() -> Router<AppState> {
    Router::new()
        .route("/retry-build", post(build_retry_handler))
        .route("/v2/build-events/{task_id}", get(build_event_get_handler))
        .route("/v2/targets/{task_id}", get(targets_get_handler))
        .route("/v2/build-state/{build_id}", get(build_state_handler))
        .route("/v2/builds/{build_id}/logs", get(build_logs_handler))
        .route(
            "/v2/latest_build_result/{task_id}",
            get(latest_build_result_handler),
        )
}

fn worker_routes() -> Router<AppState> {
    Router::new()
        .route("/orion-clients-info", post(get_orion_clients_info))
        .route(
            "/orion-client-status/{id}",
            get(get_orion_client_status_by_id),
        )
}

fn target_status_routes() -> Router<AppState> {
    Router::new()
        .route("/targets/{target_id}/logs", get(target_logs_handler))
        .route(
            "/v2/all-target-status/{task_id}",
            get(targets_status_handler),
        )
        .route(
            "/v2/target-status/{target_id}",
            get(single_target_status_handle),
        )
}

/// API endpoint for getting queue statistics
#[utoipa::path(
    get,
    path = "/queue-stats",
    tag = "System",
    responses(
        (status = 200, description = "Queue statistics", body = TaskQueueStats)
    )
)]
pub async fn queue_stats_handler(State(state): State<AppState>) -> impl IntoResponse {
    api_v2_service::queue_stats(&state).await
}

/// Health check endpoint for Orion Server
/// Returns simple health status based on database connectivity
#[utoipa::path(
    get,
    path = "/v2/health",
    tag = "System",
    responses(
        (status = 200, description = "Service is healthy", body = serde_json::Value),
        (status = 503, description = "Service is unhealthy", body = serde_json::Value)
    )
)]
pub async fn health_check_handler(State(state): State<AppState>) -> impl IntoResponse {
    api_v2_service::health_check(&state).await
}

/// Streams build output logs in real-time using Server-Sent Events (SSE)
/// Continuously monitors the log file and streams new content as it becomes available
#[utoipa::path(
    get,
    path = "/task-output/{id}",
    tag = "Task",
    params(
        ("id" = String, Path, description = "Build ID for which to stream output logs")
    ),
    responses(
        (status = 200, description = "Server-Sent Events stream of build output logs"),
        (status = 404, description = "Task output file not found")
    )
)]
pub async fn task_output_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    api_v2_service::task_output(&state, &id).await
}

/// Provides the ability to read historical task logs
/// supporting either retrieving the entire log at once or segmenting it by line count.
#[utoipa::path(
    get,
    path = "/task-history-output",
    tag = "Task",
    params(
        ("task_id" = String, Query, description = "Task ID whose log to read"),
        ("build_id" = String, Query, description = "Build ID whose log to read"),
        ("repo" = String, Query, description = "build repository path"),
        ("start" = Option<usize>, Query, description = "Start line number (0-based)"),
        ("end"  = Option<usize>, Query, description = "End line number"),
    ),
    responses(
        (status = 200, description = "History Log", body = api_model::buck2::types::LogLinesResponse),
        (status = 400, description = "Invalid parameters", body = api_model::buck2::types::LogErrorResponse),
        (status = 404, description = "Log file not found", body = api_model::buck2::types::LogErrorResponse),
        (status = 500, description = "Failed to operate log file", body = api_model::buck2::types::LogErrorResponse),
    )
)]
pub async fn task_history_output_handler(
    State(state): State<AppState>,
    Query(params): Query<TaskHistoryQuery>,
) -> Result<Json<LogLinesResponse>, (StatusCode, Json<LogErrorResponse>)> {
    api_v2_service::task_history_output(&state, &params).await
}

#[utoipa::path(
    get,
    path = "/targets/{target_id}/logs",
    tag = "TargetStatus",
    params(
        ("target_id" = String, Path, description = "Target ID whose logs to read"),
        ("type" = String, Query, description = "full | segment"),
        ("build_id" = Option<String>, Query, description = "Optional build ID to read logs from"),
        ("offset" = Option<usize>, Query, description = "Start line number for segment mode"),
        ("limit" = Option<usize>, Query, description = "Max lines for segment mode"),
    ),
    responses(
        (
            status = 200,
            description = "Target log content",
            body = api_model::buck2::types::TargetLogLinesResponse
        ),
        (status = 404, description = "Target or log not found", body = api_model::buck2::types::LogErrorResponse),
        (status = 500, description = "Failed to read log", body = api_model::buck2::types::LogErrorResponse)
    )
)]
pub async fn target_logs_handler(
    State(state): State<AppState>,
    Path(target_id): Path<String>,
    Query(params): Query<TargetLogQuery>,
) -> Result<Json<TargetLogLinesResponse>, (StatusCode, Json<LogErrorResponse>)> {
    api_v2_service::target_logs(&state, &target_id, &params).await
}

#[utoipa::path(
    post,
    path = "/v2/task",
    tag = "Task",
    request_body = TaskBuildRequest,
    responses(
        (status = 200, description = "Task created", body = serde_json::Value),
        (status = 503, description = "Queue is full", body = serde_json::Value)
    )
)]
/// Handling task creation and returns the task ID with status (immediate or queued)
pub async fn task_handler_v2(
    State(state): State<AppState>,
    Json(req): Json<TaskBuildRequest>,
) -> impl IntoResponse {
    api_v2_service::task_handler_v2(&state, req).await
}

/// Creates build tasks and returns the task ID and status (immediate or queued)
#[utoipa::path(
    post,
    path = "/task",
    tag = "Task",
    request_body = TaskBuildRequest,
    responses(
        (status = 200, description = "Task created", body = serde_json::Value),
        (status = 503, description = "Queue is full", body = serde_json::Value)
    )
)]
pub async fn task_handler(
    State(state): State<AppState>,
    Json(req): Json<TaskBuildRequest>,
) -> impl IntoResponse {
    api_v2_service::task_handler_v1(&state, req).await
}

#[utoipa::path(
    get,
    path = "/task-build-list/{id}",
    tag = "Task",
    params(
        ("id" = String, Path, description = "Task ID to get build IDs for")
    ),
    responses(
        (status = 200, description = "List of build IDs associated with the task", body = [String]),
        (status = 400, description = "Invalid task ID format", body = serde_json::Value),
        (status = 404, description = "Task not found", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    )
)]
pub async fn task_build_list_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    api_v2_service::task_build_list(&state, &id).await
}

/// Handles WebSocket upgrade requests from workers
/// Establishes bidirectional communication channel with worker nodes
async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws_service::ws_handler(ws, ConnectInfo(addr), State(state)).await
}

#[utoipa::path(
    get,
    path = "/tasks/{cl}",
    tag = "Task",
    params(
        ("cl" = i64, Path, description = "CL number to filter tasks by")
    ),
    responses(
    (status = 200, description = "All tasks with their current status", body = [TaskInfoDTO]),
    (status = 500, description = "Internal error", body = serde_json::Value)
    )
)]
/// Return all tasks with their current status (combining /cl-task and /task-status logic)
pub async fn tasks_handler(
    State(state): State<AppState>,
    Path(cl): Path<i64>,
) -> Result<Json<Vec<TaskInfoDTO>>, (StatusCode, Json<serde_json::Value>)> {
    api_v2_service::tasks_by_cl(&state, cl).await
}

#[utoipa::path(
    get,
    path = "/tasks/{task_id}/targets",
    tag = "Task",
    params(
        ("task_id" = String, Path, description = "Task ID to query targets for")
    ),
    responses(
        (status = 200, description = "Task with targets", body = TaskInfoDTO),
        (status = 400, description = "Invalid task ID", body = serde_json::Value),
        (status = 404, description = "Task not found", body = serde_json::Value),
        (status = 500, description = "Internal error", body = serde_json::Value)
    )
)]
pub async fn task_targets_handler(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskInfoDTO>, (StatusCode, Json<serde_json::Value>)> {
    api_v2_service::task_targets(&state, &task_id).await
}

#[utoipa::path(
    get,
    path = "/tasks/{task_id}/targets/summary",
    tag = "Task",
    params(
        ("task_id" = String, Path, description = "Task ID to query target summary for")
    ),
    responses(
        (status = 200, description = "Target summary", body = TargetSummaryDTO),
        (status = 400, description = "Invalid task ID", body = serde_json::Value),
        (status = 500, description = "Internal error", body = serde_json::Value)
    )
)]
pub async fn task_targets_summary_handler(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<TargetSummaryDTO>, (StatusCode, Json<serde_json::Value>)> {
    api_v2_service::task_targets_summary(&state, &task_id).await
}

/// Endpoint to retrieve paginated Orion client information.
#[utoipa::path(
    post,
    path = "/orion-clients-info",
    tag = "Worker",
    request_body = PageParams<OrionClientQuery>,
    responses(
        (status = 200, description = "Paged Orion client information", body = CommonPage<OrionClientInfo>)
    )
)]
pub async fn get_orion_clients_info(
    State(state): State<AppState>,
    Json(params): Json<PageParams<OrionClientQuery>>,
) -> Result<Json<CommonPage<OrionClientInfo>>, (StatusCode, Json<serde_json::Value>)> {
    api_v2_service::get_orion_clients_info(&state, params).await
}

/// Retrieve the current status of a specific Orion client by its ID.
#[utoipa::path(
    get,
    path = "/orion-client-status/{id}",
    tag = "Worker",
    params(
        ("id" = String, description = "Orion client Id")
    ),
    responses(
        (status = 200, description = "Orion status", body = OrionClientStatus),
        (status = 404, description = "Orion client not found", body = serde_json::Value)
    )
)]
pub async fn get_orion_client_status_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OrionClientStatus>, (StatusCode, Json<serde_json::Value>)> {
    api_v2_service::get_orion_client_status_by_id(&state, &id).await
}

/// Retry the build
#[utoipa::path(
    post,
    path = "/retry-build",
    tag = "Build",
    request_body = RetryBuildRequest,
    responses(
        (status = 200, description = "Retry created", body = serde_json::Value),
        (status = 400, description = "Invalid build ID format", body = serde_json::Value),
        (status = 404, description = "Build Id not found", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value),
        (status = 502, description = "Send to worker error", body = serde_json::Value),
        (status = 503, description = "Queue is full", body = serde_json::Value)
    )
)]
pub async fn build_retry_handler(
    State(state): State<AppState>,
    Json(req): Json<RetryBuildRequest>,
) -> impl IntoResponse {
    api_v2_service::build_retry(&state, req).await
}

#[utoipa::path(
    post,
    path = "/v2/task-retry/{id}",
    tag = "Task",
    params(("id" = String, description = "Task ID to retry task")),
    responses(
        (status = 200, description = "Task queued for retry", body = MessageResponse),
        (status = 400, description = "ID format error", body = MessageResponse),
        (status = 404, description = "Not found this task ID", body = MessageResponse),
    )
)]
pub async fn task_retry_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<MessageResponse>)> {
    api_v2_service::task_retry(&state, &id).await
}

#[utoipa::path(
    get,
    path = "/v2/task/{cl}",
    tag = "Task",
    params(("cl" = String, Path, description = "Change List")),
    responses(
        (status = 200, description = "Get task successfully", body = OrionTaskDTO),
        (status = 400, description = "Multiple tasks", body = MessageResponse),
        (status = 404, description = "Not found task", body = MessageResponse),
        (status = 500, description = "Database error", body = MessageResponse),
    )
)]
pub async fn task_get_handler(
    State(state): State<AppState>,
    Path(cl): Path<String>,
) -> Result<Json<OrionTaskDTO>, (StatusCode, Json<serde_json::Value>)> {
    api_v2_service::task_get(&state, &cl).await
}

#[utoipa::path(
    get,
    path = "/v2/build-events/{task_id}",
    tag = "Build",
    params(("task_id" = String, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Get build events successfully", body = Vec<BuildEventDTO>),
        (status = 400, description = "Bad task id", body = MessageResponse),
        (status = 404, description = "Not found task", body = MessageResponse),
        (status = 500, description = "Database error", body = MessageResponse),
    )
)]
pub async fn build_event_get_handler(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<Vec<BuildEventDTO>>, (StatusCode, Json<serde_json::Value>)> {
    api_v2_service::build_event_get(&state, &task_id).await
}

#[utoipa::path(
    get,
    path = "/v2/targets/{task_id}",
    tag = "Build",
    params(("task_id" = String, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Get targets successfully", body = Vec<BuildTargetDTO>),
        (status = 404, description = "Not found task", body = MessageResponse),
        (status = 500, description = "Internal server error", body = MessageResponse),
    )
)]
pub async fn targets_get_handler(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<Vec<BuildTargetDTO>>, (StatusCode, Json<MessageResponse>)> {
    api_v2_service::targets_get(&state, &task_id).await
}

/// Get complete log for a specific build event
#[utoipa::path(
        get,
        path = "/v2/builds/{build_id}/logs",
        tag = "Build",
        params(("build_id" = String, Path, description = "Build event ID")),
        responses(
            (status = 200, description = "Complete log content", body = api_model::buck2::types::LogLinesResponse),
            (status = 400, description = "Invalid build ID", body = LogErrorResponse),
            (status = 404, description = "Build event or log not found", body = LogErrorResponse),
            (status = 500, description = "Database or log read error", body = LogErrorResponse),
        )
    )]
pub async fn build_logs_handler(
    State(state): State<AppState>,
    Path(build_id): Path<String>,
) -> Result<Json<LogLinesResponse>, (StatusCode, Json<LogErrorResponse>)> {
    api_v2_service::build_logs(&state, &build_id).await
}

/// Get build state by build ID
#[utoipa::path(
    get,
    path = "/v2/build-state/{build_id}",
    tag = "Build",
    params(("build_id" = String, Path, description = "Build ID")),
    responses(
        (status = 200, description = "Get build state successfully", body = BuildEventState),
        (status = 404, description = "Not found build", body = MessageResponse),
    )
)]
pub async fn build_state_handler(
    State(state): State<AppState>,
    Path(build_id): Path<String>,
) -> Result<Json<BuildEventState>, (StatusCode, Json<MessageResponse>)> {
    api_v2_service::build_state(&state, &build_id).await
}

/// Get latest build result by task ID
#[utoipa::path(
    get,
    path = "/v2/latest_build_result/{task_id}",
    tag = "Build",
    params(("task_id" = String, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Get latest build result successfully", body = BuildEventState),
        (status = 404, description = "Not found task", body = MessageResponse),
    )
)]
pub async fn latest_build_result_handler(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<BuildEventState>, (StatusCode, Json<MessageResponse>)> {
    api_v2_service::latest_build_result(&state, &task_id).await
}

/// Get target status with task_id
#[utoipa::path(
    get,
    path = "/v2/all-target-status/{task_id}",
    tag = "TargetStatus",
    params(
        ("task_id" = String, Path, description = "Task ID whose target belong"),
    ),
     responses(
        (status = 200, description = "Target status"),
        (status = 404, description = "Target status not found")
    )
)]
pub async fn targets_status_handler(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<Vec<TargetStatusResponse>>, (StatusCode, String)> {
    crate::service::target_status_cache_service::targets_status_by_task_id(&state.conn, &task_id)
        .await
        .map(Json)
}

/// Get target status with target id
#[utoipa::path(
    get,
    path = "/v2/target-status/{target_id}",
    tag = "TargetStatus",
    params(
        ("target_id" = String, Path, description = "target_id ID"),
    ),
     responses(
        (status = 200, description = "Target status"),
        (status = 404, description = "Target status not found")
    )
)]
pub async fn single_target_status_handle(
    State(state): State<AppState>,
    Path(target_id): Path<String>,
) -> Result<Json<TargetStatusResponse>, (StatusCode, String)> {
    crate::service::target_status_cache_service::target_status_by_id(&state.conn, &target_id)
        .await
        .map(Json)
}

#[cfg(test)]
mod tests {
    /// Test random number generation for worker selection
    #[test]
    fn test_rng() {
        use rand::seq::IndexedRandom;

        let choices = [1, 2, 4, 8, 16, 32];
        let mut rng = rand::rng();
        println!("{:?}", choices.choose(&mut rng));
        println!("{:?}", choices.choose(&mut rng));
    }
}
