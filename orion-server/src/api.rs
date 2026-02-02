use std::{
    collections::HashMap, convert::Infallible, net::SocketAddr, ops::ControlFlow, sync::Arc,
    time::Duration,
};

use anyhow::Result;
use api_model::buck2::types::{
    LogErrorResponse, LogEvent, LogLinesResponse, LogReadMode, TargetLogLinesResponse,
    TargetLogQuery, TaskHistoryQuery,
};
use axum::{
    Json, Router,
    extract::{
        ConnectInfo, Path, Query, State, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    http::StatusCode,
    response::{IntoResponse, Sse, sse::Event},
    routing::{any, get, post},
};
use chrono::{FixedOffset, Utc};
use dashmap::DashMap;
use futures::stream::select;
use futures_util::{SinkExt, Stream, StreamExt};
use orion::ws::{TaskPhase, WSMessage};
use rand::Rng;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter as _, QueryOrder,
    QuerySelect, prelude::DateTimeUtc,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::{mpsc, mpsc::UnboundedSender, watch};
use tokio_stream::wrappers::IntervalStream;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auto_retry::AutoRetryJudger,
    log::log_service::LogService,
    model::{
        builds, targets,
        targets::{TargetState, TargetWithBuilds},
        tasks,
    },
    orion_common::model::{CommonPage, PageParams},
    scheduler::{
        self, BuildInfo, BuildRequest, TaskQueueStats, TaskScheduler, WorkerInfo, WorkerStatus,
    },
};

const RETRY_COUNT_MAX: i32 = 3;

#[derive(Debug, Serialize, Default, ToSchema)]
struct BuildResult {
    build_id: String,
    status: String,
    message: String,
}

#[derive(Debug, Serialize, Default, ToSchema)]
struct TaskResponse {
    task_id: String,
    results: Vec<BuildResult>,
}

/// Enumeration of possible task statuses
#[derive(Debug, Serialize, Default, ToSchema, Clone)]
pub enum TaskStatusEnum {
    /// Task is queued and waiting to be assigned to a worker
    Pending,
    Building,
    Interrupted, // Task was interrupted, exit code is None
    Failed,
    Completed,
    #[default]
    NotFound,
}

/// Request structure for creating a task
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TaskRequest {
    #[serde(rename = "repo", alias = "mount_path", alias = "path")]
    pub repo: String,
    pub cl_link: String,
    pub cl: i64,
    pub builds: Vec<scheduler::BuildRequest>,
}
/// Request structure for Retry a build
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RetryBuildRequest {
    pub build_id: String,
    pub cl_link: String,
    pub cl: i64,
    pub build: scheduler::BuildRequest,
}

/// Shared application state containing worker connections, database, and active builds
#[derive(Clone)]
pub struct AppState {
    pub scheduler: TaskScheduler,
    pub conn: DatabaseConnection,
    pub log_service: LogService,
}

impl AppState {
    /// Create new AppState instance
    pub fn new(
        conn: DatabaseConnection,
        queue_config: Option<crate::scheduler::TaskQueueConfig>,
        log_service: LogService,
    ) -> Self {
        let workers = Arc::new(DashMap::new());
        let active_builds = Arc::new(DashMap::new());
        let scheduler = TaskScheduler::new(conn.clone(), workers, active_builds, queue_config);

        Self {
            scheduler,
            conn,
            log_service,
        }
    }
}

/// Creates and configures all API routes
pub fn routers() -> Router<AppState> {
    Router::new()
        .route("/ws", any(ws_handler))
        .route("/task", post(task_handler))
        .route("/task-build-list/{id}", get(task_build_list_handler))
        .route("/task-output/{id}", get(task_output_handler))
        .route("/task-history-output", get(task_history_output_handler))
        .route("/targets/{target_id}/logs", get(target_logs_handler))
        .route("/tasks/{cl}", get(tasks_handler))
        .route("/tasks/{task_id}/targets", get(task_targets_handler))
        .route("/queue-stats", get(queue_stats_handler))
        .route("/orion-clients-info", post(get_orion_clients_info))
        .route(
            "/orion-client-status/{id}",
            get(get_orion_client_status_by_id),
        )
        .route("/retry-build", post(build_retry_handler))
        .route("/v2/health", get(health_check_handler))
}

/// Start queue management background task (event-driven + periodic cleanup)
pub async fn start_queue_manager(state: AppState) {
    // Start the scheduler's queue manager
    state.scheduler.start_queue_manager().await;
}

/// API endpoint for getting queue statistics
#[utoipa::path(
    get,
    path = "/queue-stats",
    responses(
        (status = 200, description = "Queue statistics", body = TaskQueueStats)
    )
)]
pub async fn queue_stats_handler(State(state): State<AppState>) -> impl IntoResponse {
    let stats = state.scheduler.get_queue_stats().await;
    (StatusCode::OK, Json(stats))
}

/// Health check endpoint for Orion Server
/// Returns simple health status based on database connectivity
#[utoipa::path(
    get,
    path = "/v2/health",
    responses(
        (status = 200, description = "Service is healthy", body = serde_json::Value),
        (status = 503, description = "Service is unhealthy", body = serde_json::Value)
    )
)]
pub async fn health_check_handler(State(state): State<AppState>) -> impl IntoResponse {
    // Simple health check: verify database connectivity
    match tasks::Entity::find().limit(1).all(&state.conn).await {
        Ok(_) => (StatusCode::OK, Json(json!({"status": "healthy"}))),
        Err(e) => {
            tracing::error!("Health check failed: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"status": "unhealthy", "error": "database connectivity check failed"})),
            )
        }
    }
}

/// Streams build output logs in real-time using Server-Sent Events (SSE)
/// Continuously monitors the log file and streams new content as it becomes available
#[utoipa::path(
    get,
    path = "/task-output/{id}",
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
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    if !state.scheduler.active_builds.contains_key(&id) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Use watch channel as stop signal for all streams
    let (stop_tx, stop_rx) = watch::channel(true);

    let log_stop_rx = stop_rx.clone();
    // Log stream with termination condition
    let log_stream = state
        .log_service
        .subscribe_for_build(id.clone())
        .map(|log_event| {
            Ok::<Event, Infallible>(Event::default().event("log").data(log_event.line))
        })
        .take_while(move |_| {
            let stop_rx = log_stop_rx.clone();
            async move { *stop_rx.borrow() }
        });

    let heart_stop_rx = stop_rx.clone();
    // Heartbeat stream every 15 seconds with termination condition
    let heartbeat_stream = IntervalStream::new(tokio::time::interval(Duration::from_secs(15)))
        .map(|_| Ok::<Event, Infallible>(Event::default().comment("heartbeat")))
        .take_while(move |_| {
            let stop_rx_clone = heart_stop_rx.clone();
            async move { *stop_rx_clone.borrow() }
        });

    // Spawn a task to watch active_builds and send stop signal when build ends
    let stop_tx_clone = stop_tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if !state.scheduler.active_builds.contains_key(&id) {
                let _ = stop_tx_clone.send(false);
                break;
            }
        }
    });

    // Merge log and heartbeat streams
    let stream = select(log_stream, heartbeat_stream);

    Ok(Sse::new(stream))
}

/// Provides the ability to read historical task logs
/// supporting either retrieving the entire log at once or segmenting it by line count.
#[utoipa::path(
    get,
    path = "/task-history-output",
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
    // Determine which read method to call
    let log_result = if matches!((params.start, params.end), (None, None)) {
        state
            .log_service
            .read_full_log(&params.task_id, &params.repo, &params.build_id)
            .await
    } else {
        // Unwrap start/end, default to 0 if needed
        let start = params.start.unwrap_or(0);
        let end = params.end.unwrap_or(usize::MAX);
        state
            .log_service
            .read_log_range(&params.task_id, &params.repo, &params.build_id, start, end)
            .await
    };

    // Handle result
    let log_content = match log_result {
        Ok(content) => content,
        Err(e) => {
            tracing::error!("read log failed: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LogErrorResponse {
                    message: "Failed to read log file".to_string(),
                }),
            ));
        }
    };

    // Split the content into lines and count them
    let lines: Vec<String> = log_content.lines().map(str::to_string).collect();
    let len = lines.len();

    Ok(Json(LogLinesResponse { data: lines, len }))
}

#[utoipa::path(
    get,
    path = "/targets/{target_id}/logs",
    params(
        ("target_id" = String, Path, description = "Target ID whose logs to read"),
        ("type" = String, Query, description = "full | segment"),
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
    let target_uuid = match target_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(LogErrorResponse {
                    message: "Invalid target id".to_string(),
                }),
            ));
        }
    };

    let target_model = match targets::Entity::find_by_id(target_uuid)
        .one(&state.conn)
        .await
    {
        Ok(Some(target)) => target,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(LogErrorResponse {
                    message: "Target not found".to_string(),
                }),
            ));
        }
        Err(err) => {
            tracing::error!("Failed to load target {}: {}", target_id, err);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LogErrorResponse {
                    message: "Failed to read target".to_string(),
                }),
            ));
        }
    };

    let build_model = match builds::Entity::find()
        .filter(builds::Column::TargetId.eq(target_uuid))
        .order_by_desc(builds::Column::EndAt)
        .order_by_desc(builds::Column::CreatedAt)
        .one(&state.conn)
        .await
    {
        Ok(Some(build)) => build,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(LogErrorResponse {
                    message: "No builds for target".to_string(),
                }),
            ));
        }
        Err(err) => {
            tracing::error!("Failed to load build for target {}: {}", target_uuid, err);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LogErrorResponse {
                    message: "Failed to load build".to_string(),
                }),
            ));
        }
    };

    let repo_segment = LogService::last_segment(&build_model.repo);
    let log_result = if matches!(params.r#type, LogReadMode::Segment) {
        let offset = params.offset.unwrap_or(0);
        let limit = params.limit.unwrap_or(200);
        state
            .log_service
            .read_log_range(
                &target_model.task_id.to_string(),
                &repo_segment,
                &build_model.id.to_string(),
                offset,
                offset + limit,
            )
            .await
    } else {
        state
            .log_service
            .read_full_log(
                &target_model.task_id.to_string(),
                &repo_segment,
                &build_model.id.to_string(),
            )
            .await
    };

    match log_result {
        Ok(content) => {
            let lines: Vec<String> = content.lines().map(str::to_string).collect();
            let len = lines.len();
            Ok(Json(TargetLogLinesResponse {
                data: lines,
                len,
                build_id: build_model.id.to_string(),
            }))
        }
        Err(e) => {
            tracing::error!(
                "Failed to read logs for target {} build {}: {}",
                target_uuid,
                build_model.id,
                e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LogErrorResponse {
                    message: "Failed to read log".to_string(),
                }),
            ))
        }
    }
}

/// Creates build tasks and returns the task ID and status (immediate or queued)
#[utoipa::path(
    post,
    path = "/task",
    request_body = TaskRequest,
    responses(
        (status = 200, description = "Task created", body = serde_json::Value),
        (status = 503, description = "Queue is full", body = serde_json::Value)
    )
)]
pub async fn task_handler(
    State(state): State<AppState>,
    Json(req): Json<TaskRequest>,
) -> impl IntoResponse {
    // create task id
    let task_id = Uuid::now_v7();

    // Process each build
    let mut results = Vec::with_capacity(req.builds.len());
    // Insert task into the database using the model's insert method
    if let Err(err) = tasks::Model::insert_task(
        task_id,
        req.cl,
        None,
        None,
        chrono::Utc::now().into(),
        &state.conn,
    )
    .await
    {
        tracing::error!("Failed to insert task into DB: {}", err);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "message": format!("Failed to insert task into database: {}", err)
            })),
        )
            .into_response();
    }

    for build in &req.builds {
        // Check if there are idle workers available
        if state.scheduler.has_idle_workers() {
            // Have idle workers, directly dispatch task (keep original logic)
            let result: BuildResult = handle_immediate_task_dispatch(
                state.clone(),
                task_id,
                &req.cl_link,
                &req.repo,
                req.cl,
                build.clone(),
            )
            .await;
            results.push(result);
        } else {
            // No idle workers, add task to queue
            match state
                .scheduler
                .enqueue_task(
                    task_id,
                    &req.cl_link,
                    build.clone(),
                    req.repo.clone(),
                    req.cl,
                    0,
                )
                .await
            {
                Ok(build_id) => {
                    tracing::info!("Build {}/{} queued for later processing", task_id, build_id);
                    let result: BuildResult = BuildResult {
                        build_id: build_id.to_string(),
                        status: "queued".to_string(),
                        message: "Task queued for processing when workers become available"
                            .to_string(),
                    };
                    results.push(result);
                }
                Err(e) => {
                    tracing::warn!("Failed to queue task: {}", e);
                    let result: BuildResult = BuildResult {
                        build_id: "".to_string(),
                        status: "error".to_string(),
                        message: format!("Unable to queue task: {}", e),
                    };
                    results.push(result);
                }
            }
        }
    }

    (
        StatusCode::OK,
        Json(TaskResponse {
            task_id: task_id.to_string(),
            results,
        }),
    )
        .into_response()
}

async fn handle_immediate_task_dispatch(
    state: AppState,
    task_id: Uuid,
    cl_link: &str,
    repo: &str,
    cl: i64,
    req: BuildRequest,
) -> BuildResult {
    // Find all idle workers
    let idle_workers = state.scheduler.get_idle_workers();

    // Return error if no workers are available (this shouldn't happen theoretically since we already checked)
    if idle_workers.is_empty() {
        return BuildResult {
            build_id: "".to_string(),
            status: "error".to_string(),
            message: "No available workers at the moment".to_string(),
        };
    }

    // Randomly select an idle worker
    let chosen_index = {
        let mut rng = rand::rng();
        rng.random_range(0..idle_workers.len())
    };
    let chosen_id = idle_workers[chosen_index].clone();
    let build_id = Uuid::now_v7();
    let target_path = req.target_path();

    let target_model = match state.scheduler.ensure_target(task_id, &target_path).await {
        Ok(target) => target,
        Err(err) => {
            tracing::error!("Failed to prepare target {}: {}", target_path, err);
            return BuildResult {
                build_id: "".to_string(),
                status: "error".to_string(),
                message: format!("Failed to prepare target {}", target_path),
            };
        }
    };
    let start_at = chrono::Utc::now();
    let start_at_tz = start_at.with_timezone(&FixedOffset::east_opt(0).unwrap());

    // Mark target as building
    if let Err(e) = targets::update_state(
        &state.conn,
        target_model.id,
        TargetState::Building,
        Some(start_at_tz),
        None,
        None,
    )
    .await
    {
        tracing::error!("Failed to update target state to Building: {}", e);
    }

    // Create build information structure
    let build_info = BuildInfo {
        task_id: task_id.to_string(),
        build_id: build_id.to_string(),
        target_id: target_model.id.to_string(),
        target_path: target_path.clone(),
        repo: repo.to_string(),
        changes: req.changes.clone(),
        start_at,
        cl: cl.to_string(),
        _worker_id: chosen_id.clone(),
        auto_retry_judger: AutoRetryJudger::new(),
        retry_count: 0,
    };

    // Use the model's insert_build method for direct insertion
    if let Err(err) = builds::Model::insert_build(
        build_id,
        task_id,
        target_model.id,
        repo.to_string(),
        &state.conn,
    )
    .await
    {
        tracing::error!("Failed to insert builds into DB: {}", err);
        return BuildResult {
            build_id: "".to_string(),
            status: "error".to_string(),
            message: format!("Failed to insert builds into database: {}", err),
        };
    }
    println!("insert build");

    // Create WebSocket message for the worker (use first build's args)
    let msg: WSMessage = WSMessage::Task {
        id: build_id.to_string(),
        repo: repo.to_string(),
        changes: req.changes.clone(),
        cl_link: cl_link.to_string(),
    };

    // Send task to the selected worker
    if let Some(mut worker) = state.scheduler.workers.get_mut(&chosen_id)
        && worker.sender.send(msg).is_ok()
    {
        worker.status = WorkerStatus::Busy {
            task_id: build_id.to_string(),
            phase: None,
        };
        state
            .scheduler
            .active_builds
            .insert(build_id.to_string(), build_info);
        tracing::info!(
            "Build {}/{} dispatched immediately to worker {}",
            task_id,
            build_id,
            chosen_id
        );
        return BuildResult {
            build_id: build_id.to_string(),
            status: "dispatched".to_string(),
            message: format!("Build dispatched to worker {}", chosen_id),
        };
    }

    // If we reach here, sending failed
    BuildResult {
        build_id: "".to_string(),
        status: "error".to_string(),
        message: "Failed to dispatch task to worker".to_string(),
    }
}

#[utoipa::path(
    get,
    path = "/task-build-list/{id}",
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
    let db = &state.conn;

    let task_id = match id.parse::<uuid::Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"message": "Invalid task ID format"})),
            )
                .into_response();
        }
    };

    match tasks::Model::get_builds_by_task_id(task_id, db).await {
        Some(build_ids) => {
            let build_ids_str: Vec<String> =
                build_ids.into_iter().map(|uuid| uuid.to_string()).collect();
            (StatusCode::OK, Json(build_ids_str)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"message": "Task not found"})),
        )
            .into_response(),
    }
}

/// Handles WebSocket upgrade requests from workers
/// Establishes bidirectional communication channel with worker nodes
async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("{addr} connected. Waiting for registration...");
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

/// Manages WebSocket connection lifecycle for worker communication
/// Handles message sending/receiving and connection cleanup
async fn handle_socket(socket: WebSocket, who: SocketAddr, state: AppState) {
    let (tx, mut rx) = mpsc::unbounded_channel::<WSMessage>();
    let mut worker_id: Option<String> = None;

    let (mut sender, mut receiver) = socket.split();

    // Task for sending messages to the worker
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let msg_str = serde_json::to_string(&msg).unwrap();
            if sender
                .send(Message::Text(Utf8Bytes::from(msg_str)))
                .await
                .is_err()
            {
                tracing::warn!("Failed to send message to {who}, client disconnected.");
                break;
            }
        }
    });

    let state_clone = state.clone();
    let tx_clone = tx.clone();

    // Task for receiving messages from the worker
    let recv_task = tokio::spawn(async move {
        let mut worker_id_inner: Option<String> = None;
        while let Some(Ok(msg)) = receiver.next().await {
            if process_message(msg, who, &state_clone, &mut worker_id_inner, &tx_clone)
                .await
                .is_break()
            {
                break;
            }
        }
        worker_id_inner
    });

    tokio::select! {
        _ = send_task => { },
        result = recv_task => {
            if let Ok(final_worker_id) = result {
                worker_id = final_worker_id;
            }
        },
    }

    // Cleanup worker connection when socket closes
    if let Some(id) = &worker_id {
        tracing::info!("Cleaning up for worker: {id} from {who}.");
        state.scheduler.workers.remove(id);
    } else {
        tracing::info!("Cleaning up unregistered connection from {who}.");
    }

    tracing::info!("Websocket context {who} destroyed");
}

/// Processes individual WebSocket messages from workers
/// Handles registration, heartbeats, build output, and completion messages
async fn process_message(
    msg: Message,
    who: SocketAddr,
    state: &AppState,
    worker_id: &mut Option<String>,
    tx: &UnboundedSender<WSMessage>,
) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            let ws_msg: Result<WSMessage, _> = serde_json::from_str(&t);
            if let Err(e) = ws_msg {
                tracing::warn!("Failed to parse message from {who}: {e}");
                return ControlFlow::Continue(());
            }
            let ws_msg = ws_msg.unwrap();

            // Handle worker registration (must be first message)
            if worker_id.is_none() {
                if let WSMessage::Register {
                    id,
                    hostname,
                    orion_version,
                } = ws_msg
                {
                    tracing::info!("Worker from {who} registered as: {id}");
                    state.scheduler.workers.insert(
                        id.clone(),
                        WorkerInfo {
                            sender: tx.clone(),
                            status: WorkerStatus::Idle,
                            last_heartbeat: chrono::Utc::now(),
                            start_time: chrono::Utc::now(),
                            hostname,
                            orion_version,
                        },
                    );
                    *worker_id = Some(id);

                    // After new worker registration, notify to process queued tasks
                    state.scheduler.notify_task_available();
                } else {
                    tracing::error!(
                        "First message from {who} was not Register. Closing connection."
                    );
                    return ControlFlow::Break(());
                }
                return ControlFlow::Continue(());
            }

            // Process messages from registered workers
            let current_worker_id = worker_id.as_ref().unwrap();
            match ws_msg {
                WSMessage::Register { .. } => {
                    tracing::warn!(
                        "Worker {current_worker_id} sent Register message again. Ignoring."
                    );
                }
                WSMessage::Heartbeat => {
                    if let Some(mut worker) = state.scheduler.workers.get_mut(current_worker_id) {
                        worker.last_heartbeat = chrono::Utc::now();
                        tracing::debug!("Received heartbeat from {current_worker_id}");

                        // If the worker was previously in Error state, a successful heartbeat now restores it to Idle.
                        if let WorkerStatus::Error(_) = worker.status {
                            worker.status = WorkerStatus::Idle;
                            tracing::info!(
                                "Worker {current_worker_id} recovered from Error to Idle via heartbeat."
                            );
                        }
                    }
                }
                WSMessage::BuildOutput { id, output } => {
                    // Write build output to the associated log file
                    if let Some(build_info) = state.scheduler.active_builds.get(&id) {
                        let log_event = LogEvent {
                            task_id: build_info.task_id.clone(),
                            repo_name: LogService::last_segment(&build_info.repo.clone())
                                .to_string(),
                            build_id: build_info.build_id.clone(),
                            line: output.clone(),
                            is_end: false,
                        };
                        // Publish the log event to the log stream
                        state.log_service.publish(log_event.clone());

                        // Debug output showing the published log
                        tracing::debug!(
                            "Published log for build_id {} (task: {}, repo: {}): {}",
                            id,
                            build_info.task_id,
                            build_info.repo,
                            output
                        );
                    } else {
                        tracing::warn!("Received output for unknown task: {}", id);
                    }

                    // Judge auto retry by output
                    if let Some(mut build_info) = state.scheduler.active_builds.get_mut(&id) {
                        build_info.auto_retry_judger.judge_by_output(&output);
                    }
                }
                WSMessage::BuildComplete {
                    id,
                    success,
                    exit_code,
                    message,
                } => {
                    // Handle build completion
                    tracing::info!(
                        "Build {id} completed by worker {current_worker_id} with exit code: {exit_code:?}"
                    );

                    // Get build information
                    let (
                        mut auto_retry_judger,
                        mut retry_count,
                        repo,
                        changes,
                        cl_link,
                        task_id,
                        target_id,
                        _target_path,
                    ) = if let Some(build_info) = state.scheduler.active_builds.get(&id) {
                        (
                            build_info.auto_retry_judger.clone(),
                            build_info.retry_count,
                            build_info.repo.clone(),
                            build_info.changes.clone(),
                            build_info.cl.clone(),
                            build_info.task_id.clone(),
                            build_info.target_id.clone(),
                            build_info.target_path.clone(),
                        )
                    } else {
                        tracing::error!("Not found build {id}");
                        return ControlFlow::Continue(());
                    };

                    // Judge auto retry by exit code
                    auto_retry_judger.judge_by_exit_code(exit_code.unwrap_or(0));

                    let can_auto_retry = auto_retry_judger.get_can_auto_retry();

                    if can_auto_retry && retry_count < RETRY_COUNT_MAX {
                        tracing::info!("Build {id} will retry, current retry count: {retry_count}");

                        // Add retry count
                        retry_count += 1;

                        // Update build information
                        if let Some(mut build_info) = state.scheduler.active_builds.get_mut(&id) {
                            build_info.retry_count = retry_count;
                            // New AutoRetryJudger
                            build_info.auto_retry_judger = AutoRetryJudger::new();
                        }

                        // Update database
                        let _ = builds::Entity::update_many()
                            .set(builds::ActiveModel {
                                retry_count: Set(retry_count),
                                ..Default::default()
                            })
                            .filter(builds::Column::Id.eq(id.parse::<uuid::Uuid>().unwrap()))
                            .exec(&state.conn)
                            .await;

                        // Send task to this worker
                        let msg = WSMessage::Task {
                            id: id.clone(),
                            repo: repo.clone(),
                            cl_link,
                            changes,
                        };
                        if let Some(worker) = state.scheduler.workers.get_mut(current_worker_id)
                            && worker.sender.send(msg).is_ok()
                        {
                            tracing::info!("Retry build: {}, worker: {}", id, current_worker_id);
                            return ControlFlow::Continue(());
                        }
                    }

                    // Send final log event
                    let log_event = LogEvent {
                        task_id: task_id.clone(),
                        repo_name: LogService::last_segment(&repo).to_string(),
                        build_id: id.clone(),
                        line: String::from(""),
                        is_end: true,
                    };
                    state.log_service.publish(log_event);

                    // Remove from active
                    state.scheduler.active_builds.remove(&id);

                    // Update database with final state
                    let end_at = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
                    let _ = builds::Entity::update_many()
                        .set(builds::ActiveModel {
                            exit_code: Set(exit_code),
                            end_at: Set(Some(end_at)),
                            retry_count: Set(retry_count),
                            ..Default::default()
                        })
                        .filter(builds::Column::Id.eq(id.parse::<uuid::Uuid>().unwrap()))
                        .exec(&state.conn)
                        .await;

                    // Update target state
                    let target_uuid = target_id.parse::<Uuid>().ok();
                    let target_state = match (success, exit_code) {
                        (true, Some(0)) => TargetState::Completed,
                        (_, None) => TargetState::Interrupted,
                        _ => TargetState::Failed,
                    };
                    let mut error_summary = None;
                    if matches!(target_state, TargetState::Failed) {
                        let repo_segment = LogService::last_segment(&repo);
                        if let Ok(log_content) = state
                            .log_service
                            .read_full_log(&task_id, &repo_segment, &id.clone())
                            .await
                        {
                            error_summary = find_caused_by_next_line_in_content(&log_content).await;
                        }
                    }
                    if let Some(target_uuid) = target_uuid {
                        if let Err(e) = targets::update_state(
                            &state.conn,
                            target_uuid,
                            target_state,
                            None,
                            Some(end_at),
                            error_summary,
                        )
                        .await
                        {
                            tracing::error!(
                                "Failed to update target state for build {}: {}",
                                id,
                                e
                            );
                        }
                    } else {
                        tracing::warn!("Unable to parse target id {} for build {}", target_id, id);
                    }

                    // Mark the worker as idle or error depending on whether the task succeeds.
                    if let Some(mut worker) = state.scheduler.workers.get_mut(current_worker_id) {
                        worker.status = if success {
                            WorkerStatus::Idle
                        } else {
                            WorkerStatus::Error(message)
                        };
                    }

                    // Notify scheduler to process queued tasks
                    state.scheduler.notify_task_available();
                }
                WSMessage::TaskPhaseUpdate { id, phase } => {
                    tracing::info!(
                        "Task phase updated by orion worker {current_worker_id} with: {phase:?}"
                    );
                    if let Some(mut worker) = state.scheduler.workers.get_mut(current_worker_id) {
                        if let WorkerStatus::Busy { task_id, .. } = &worker.status {
                            if task_id == &id {
                                worker.status = WorkerStatus::Busy {
                                    task_id: id,
                                    phase: Some(phase),
                                };
                            } else {
                                tracing::warn!(
                                    "Ignoring TaskPhaseUpdate for worker {current_worker_id}: \
                                     task_id mismatch (expected {task_id}, got {id})"
                                );
                            }
                        } else {
                            tracing::warn!(
                                "Ignoring TaskPhaseUpdate for worker {current_worker_id}: \
                                 worker not in Busy state (current status: {:?})",
                                worker.status
                            );
                        }
                    } else {
                        tracing::warn!(
                            "Ignoring TaskPhaseUpdate: unknown worker {current_worker_id}"
                        );
                    }
                }
                _ => {}
            }
        }
        Message::Close(_) => {
            tracing::info!("Client {who} sent close message.");
            if let Some(id) = worker_id.take()
                && let Some(mut worker) = state.scheduler.workers.get_mut(&id)
            {
                worker.status = WorkerStatus::Lost;
                tracing::info!("Worker {id} marked as Lost due to connection close");
            }

            return ControlFlow::Break(());
        }
        _ => {}
    }
    ControlFlow::Continue(())
}

/// Data transfer object for build information in API responses
#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct BuildDTO {
    pub id: String,
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    pub exit_code: Option<i32>,
    pub start_at: String,
    pub end_at: Option<String>,
    pub repo: String,
    pub target: String,
    pub args: Option<Value>,
    pub output_file: String,
    pub created_at: String,
    pub retry_count: i32,
    pub status: TaskStatusEnum,
    pub cause_by: Option<String>,
}

impl BuildDTO {
    /// Converts a database model to a DTO for API responses.
    /// `target` is optional; empty string means target path missing (for compat).
    pub fn from_model(
        model: builds::Model,
        target: Option<&targets::Model>,
        status: TaskStatusEnum,
    ) -> Self {
        let target_path = target.map(|t| t.target_path.clone()).unwrap_or_default();
        Self {
            id: model.id.to_string(),
            task_id: model.task_id.to_string(),
            target_id: target.map(|t| t.id.to_string()),
            exit_code: model.exit_code,
            start_at: model.start_at.with_timezone(&Utc).to_rfc3339(),
            end_at: model.end_at.map(|dt| dt.with_timezone(&Utc).to_rfc3339()),
            repo: model.repo,
            target: target_path,
            args: model.args.map(|v| json!(v)),
            output_file: model.output_file,
            created_at: model.created_at.with_timezone(&Utc).to_rfc3339(),
            retry_count: model.retry_count,
            status,
            cause_by: None,
        }
    }

    /// Determine build status based on database fields and active builds
    pub fn determine_status(model: &builds::Model, is_active: bool) -> TaskStatusEnum {
        if is_active {
            TaskStatusEnum::Building
        } else if model.end_at.is_none() {
            // Not in active_builds and end_at is None => still queued (pending)
            TaskStatusEnum::Pending
        } else if model.exit_code.is_none() {
            TaskStatusEnum::Interrupted
        } else if model.exit_code == Some(0) {
            TaskStatusEnum::Completed
        } else {
            TaskStatusEnum::Failed
        }
    }
}

pub type TargetDTO = targets::TargetWithBuilds<BuildDTO>;

/// Task information including current status
#[derive(Debug, Serialize, ToSchema)]
pub struct TaskInfoDTO {
    pub task_id: String,
    pub cl_id: i64,
    pub task_name: Option<String>,
    pub template: Option<serde_json::Value>,
    pub created_at: String,
    pub build_list: Vec<BuildDTO>,
    pub targets: Vec<TargetDTO>,
}

impl TaskInfoDTO {
    fn from_model(model: tasks::Model, build_list: Vec<BuildDTO>, targets: Vec<TargetDTO>) -> Self {
        Self {
            task_id: model.id.to_string(),
            cl_id: model.cl_id,
            task_name: model.task_name,
            template: model.template,
            created_at: model.created_at.with_timezone(&Utc).to_rfc3339(),
            build_list,
            targets,
        }
    }
}

#[utoipa::path(
    get,
    path = "/tasks/{cl}",
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
    let db = &state.conn;
    let active_builds = state.scheduler.active_builds.clone();

    match tasks::Entity::find()
        .filter(tasks::Column::ClId.eq(cl))
        .all(db)
        .await
    {
        Ok(task_models) => {
            let mut tasks: Vec<TaskInfoDTO> = Vec::new();

            for m in task_models {
                tasks.push(assemble_task_info(m, &state, &active_builds).await);
            }

            Ok(Json(tasks))
        }
        Err(e) => {
            tracing::error!("Failed to fetch tasks: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"message": "Failed to fetch tasks"})),
            ))
        }
    }
}

async fn assemble_task_info(
    task: tasks::Model,
    state: &AppState,
    active_builds: &Arc<DashMap<String, BuildInfo>>,
) -> TaskInfoDTO {
    let target_models = targets::Entity::find()
        .filter(targets::Column::TaskId.eq(task.id))
        .all(&state.conn)
        .await
        .unwrap_or_else(|_| vec![]);

    let build_models = builds::Entity::find()
        .filter(builds::Column::TaskId.eq(task.id))
        .all(&state.conn)
        .await
        .unwrap_or_else(|_| vec![]);

    let target_map: HashMap<Uuid, targets::Model> =
        target_models.iter().cloned().map(|t| (t.id, t)).collect();

    let mut build_list: Vec<BuildDTO> = Vec::new();
    let mut target_build_map: HashMap<Uuid, Vec<BuildDTO>> = HashMap::new();
    for build_model in build_models {
        let build_id_str = build_model.id.to_string();
        let is_active = active_builds.contains_key(&build_id_str);
        let status = BuildDTO::determine_status(&build_model, is_active);
        let mut dto = BuildDTO::from_model(
            build_model.clone(),
            target_map.get(&build_model.target_id),
            status.clone(),
        );

        // Prefer persisted error summary; avoid reading full logs in the task summary path.
        if matches!(status, TaskStatusEnum::Failed)
            && let Some(t) = target_map.get(&build_model.target_id)
            && let Some(summary) = &t.error_summary
        {
            dto.cause_by = Some(summary.clone());
        }

        target_build_map
            .entry(build_model.target_id)
            .or_default()
            .push(dto.clone());
        build_list.push(dto);
    }

    let mut target_list: Vec<TargetDTO> = Vec::new();
    for target in target_models {
        let builds = target_build_map.remove(&target.id).unwrap_or_default();
        target_list.push(TargetWithBuilds::from_model(target, builds));
    }

    TaskInfoDTO::from_model(task, build_list, target_list)
}

async fn find_caused_by_next_line_in_content(content: &str) -> Option<String> {
    let mut last_was_caused = false;

    for line in content.lines() {
        if last_was_caused {
            return Some(line.to_string());
        }

        if line.trim() == "Caused by:" {
            last_was_caused = true;
        }
    }

    None
}

#[utoipa::path(
    get,
    path = "/tasks/{task_id}/targets",
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
    let task_uuid = task_id.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"message": "Invalid task ID"})),
        )
    })?;

    let task_model = tasks::Entity::find_by_id(task_uuid)
        .one(&state.conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch task {}: {}", task_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"message": "Failed to fetch task"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"message": "Task not found"})),
            )
        })?;

    let info = assemble_task_info(task_model, &state, &state.scheduler.active_builds).await;
    Ok(Json(info))
}

// Orion client information
#[derive(Debug, Serialize, ToSchema)]
pub struct OrionClientInfo {
    pub client_id: String,
    pub hostname: String,
    pub orion_version: String,
    #[schema(value_type = String, format = "date-time")]
    pub start_time: DateTimeUtc,
    #[schema(value_type = String, format = "date-time")]
    pub last_heartbeat: DateTimeUtc,
}

impl OrionClientInfo {
    fn from_worker(client_id: impl Into<String>, worker: &WorkerInfo) -> Self {
        Self {
            client_id: client_id.into(),
            hostname: worker.hostname.clone(),
            orion_version: worker.orion_version.clone(),
            start_time: worker.start_time,
            last_heartbeat: worker.last_heartbeat,
        }
    }
}

// Orion client status
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub enum CoreWorkerStatus {
    Idle,
    Busy,
    Error,
    Lost,
}

/// Additional query parameters for querying Orion clients.
/// When no extra conditions are required, this struct can be left empty.
#[derive(Debug, Deserialize, ToSchema, Clone)]
pub struct OrionClientQuery {
    pub hostname: Option<String>,
    pub status: Option<CoreWorkerStatus>,
    pub phase: Option<TaskPhase>, // Only in Busy status
}

/// Endpoint to retrieve paginated Orion client information.
#[utoipa::path(
    post,
    path = "/orion-clients-info",
    request_body = PageParams<OrionClientQuery>,
    responses(
        (status = 200, description = "Paged Orion client information", body = CommonPage<OrionClientInfo>)
    )
)]
pub async fn get_orion_clients_info(
    State(state): State<AppState>,
    Json(params): Json<PageParams<OrionClientQuery>>,
) -> Result<Json<CommonPage<OrionClientInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let pagination = params.pagination;
    let query = params.additional.clone();

    let page = pagination.page.max(1);
    // per_page must be in 1..=100
    let per_page = pagination.per_page.clamp(1u64, 100);
    let offset = (page - 1) * per_page;

    let mut total: u64 = 0;
    let mut items: Vec<OrionClientInfo> = Vec::with_capacity(per_page as usize);

    for entry in state.scheduler.workers.iter() {
        let matches = query
            .hostname
            .as_ref()
            .is_none_or(|h| entry.value().hostname.contains(h))
            && query
                .status
                .as_ref()
                .is_none_or(|s| entry.value().status.status_type() == *s)
            && query.phase.as_ref().is_none_or(|p| {
                matches!(
                    entry.value().status,
                    WorkerStatus::Busy { phase: Some(ref x), .. } if *x == *p
                )
            });

        if matches {
            total += 1;

            if total > offset && items.len() < per_page as usize {
                items.push(OrionClientInfo::from_worker(
                    entry.key().clone(),
                    entry.value(),
                ));
            }
        }
    }

    Ok(Json(CommonPage { total, items }))
}

// Orion client status
#[derive(Debug, Serialize, ToSchema)]
pub struct OrionClientStatus {
    /// Core (Idle / Busy / Error / Lost)
    pub core_status: CoreWorkerStatus,
    /// Only when building
    pub phase: Option<TaskPhase>,
    /// Only when error
    pub error_message: Option<String>,
}

impl OrionClientStatus {
    pub fn from_worker_status(worker: &WorkerInfo) -> Self {
        match &worker.status {
            WorkerStatus::Idle => Self {
                core_status: CoreWorkerStatus::Idle,
                phase: None,
                error_message: None,
            },

            WorkerStatus::Busy { phase, .. } => Self {
                core_status: CoreWorkerStatus::Busy,
                phase: phase.clone(),
                error_message: None,
            },

            WorkerStatus::Error(msg) => Self {
                core_status: CoreWorkerStatus::Error,
                phase: None,
                error_message: Some(msg.clone()),
            },

            WorkerStatus::Lost => Self {
                core_status: CoreWorkerStatus::Lost,
                phase: None,
                error_message: None,
            },
        }
    }
}

/// Retrieve the current status of a specific Orion client by its ID.
#[utoipa::path(
    get,
    path = "/orion-client-status/{id}",
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
    let worker = state.scheduler.workers.get(&id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "message": "Orion client not found"
            })),
        )
    })?;

    let status = OrionClientStatus::from_worker_status(&worker);

    Ok(Json(status))
}

/// Retry the build
#[utoipa::path(
    post,
    path = "/retry-build",
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
    let db = &state.conn;

    let build_id = match req.build_id.parse::<uuid::Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"message": "Invalid build ID format"})),
            )
                .into_response();
        }
    };

    if state.scheduler.active_builds.contains_key(&req.build_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"message": "The build already exists"})),
        )
            .into_response();
    }

    let build = match builds::Entity::find_by_id(build_id).one(db).await {
        Ok(o) => match o {
            Some(build) => build,
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"message": "Build not found"})),
                )
                    .into_response();
            }
        },
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"message": "Database find failed"})),
            )
                .into_response();
        }
    };

    let retry_count = build.retry_count + 1;
    let target_model = match targets::Entity::find_by_id(build.target_id).one(db).await {
        Ok(Some(target)) => target,
        Ok(None) => {
            tracing::error!("Target not found for build {}", build.id);
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"message": "Target not found for build"})),
            )
                .into_response();
        }
        Err(err) => {
            tracing::error!("Failed to load target for build {}: {}", build.id, err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"message": "Database find failed"})),
            )
                .into_response();
        }
    };

    let idle_workers = state.scheduler.get_idle_workers();
    if idle_workers.is_empty() {
        let mut build_req = req.build.clone();
        if build_req.target.is_none() {
            build_req.target = Some(target_model.target_path.clone());
        }
        // Generate a new build id for queued retry to avoid PK conflict with existing build.
        let new_build_id = Uuid::now_v7();
        // No idle workers, add task to queue
        match state
            .scheduler
            .enqueue_task_with_build_id(
                new_build_id,
                build.task_id,
                &req.cl_link,
                build_req,
                build.repo.clone(),
                req.cl,
                retry_count,
            )
            .await
        {
            Ok(()) => {
                tracing::info!("Build {} queued for later processing", build.id);
                (
                    StatusCode::OK,
                    Json(serde_json::json!({"message": "Build queued for later processing"})),
                )
                    .into_response()
            }
            Err(e) => {
                tracing::warn!("Failed to queue retry build: {}", e);
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({"message": "No available workers at the moment"})),
                )
                    .into_response()
            }
        }
    } else if immediate_work(
        &state,
        build_id,
        &idle_workers,
        &build,
        &target_model,
        &req,
        retry_count,
    )
    .await
    {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Build retry dispatched immediately to worker"
            })),
        )
            .into_response()
    } else {
        tracing::warn!(
            "Failed to dispatch build {} retry to worker; worker missing or send failed",
            build.id,
        );
        (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "message": "Failed to dispatch build retry to worker"
            })),
        )
            .into_response()
    }
}

async fn immediate_work(
    state: &AppState,
    build_id: Uuid,
    idle_workers: &[String],
    build: &builds::Model,
    target: &targets::Model,
    req: &RetryBuildRequest,
    retry_count: i32,
) -> bool {
    // Randomly select an idle worker
    let chosen_index = {
        let mut rng = rand::rng();
        rng.random_range(0..idle_workers.len())
    };
    let chosen_id = idle_workers[chosen_index].clone();

    let start_at = chrono::Utc::now();
    // Create build information
    let build_info = BuildInfo {
        task_id: build.task_id.to_string(),
        build_id: build.id.to_string(),
        target_id: target.id.to_string(),
        target_path: target.target_path.clone(),
        repo: build.repo.to_string(),
        changes: req.build.changes.clone(),
        start_at,
        cl: req.cl.to_string(),
        _worker_id: chosen_id.clone(),
        auto_retry_judger: AutoRetryJudger::new(),
        retry_count,
    };

    // Send build to worker
    let msg: WSMessage = WSMessage::Task {
        id: build.id.to_string(),
        repo: build.repo.to_string(),
        changes: req.build.changes.clone(),
        cl_link: req.cl_link.to_string(),
    };
    if let Some(mut worker) = state.scheduler.workers.get_mut(&chosen_id)
        && worker.sender.send(msg).is_ok()
    {
        worker.status = WorkerStatus::Busy {
            task_id: build_id.to_string(),
            phase: None,
        };
        if let Err(e) = targets::update_state(
            &state.conn,
            target.id,
            TargetState::Building,
            Some(start_at.with_timezone(&FixedOffset::east_opt(0).unwrap())),
            None,
            None,
        )
        .await
        {
            tracing::error!("Failed to update target state to Building: {}", e);
        }
        // Insert active build
        state
            .scheduler
            .active_builds
            .insert(build.id.to_string(), build_info);
        tracing::info!(
            "Build {} retry dispatched immediately to worker {}",
            build.id,
            chosen_id
        );
        true
    } else {
        tracing::warn!(
            "Failed to dispatch build {} retry to worker {}; worker missing or send failed",
            build.id,
            chosen_id
        );
        false
    }
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
