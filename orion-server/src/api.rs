use crate::model::builds;
use crate::scheduler::{
    BuildInfo, BuildRequest, TaskQueueStats, TaskScheduler, WorkerInfo, WorkerStatus,
    create_log_file, get_build_log_dir,
};
use axum::{
    Json, Router,
    extract::{
        ConnectInfo, Path, State, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    http::StatusCode,
    response::{IntoResponse, Sse, sse::Event, sse::KeepAlive},
    routing::{any, get},
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures_util::{SinkExt, Stream, StreamExt};
use orion::ws::WSMessage;
use rand::Rng;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter as _,
};
use serde::Serialize;
use std::convert::Infallible;
use std::io::Write;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt};
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;
use utoipa::ToSchema;
use uuid::Uuid;

/// Enumeration of possible task statuses
#[derive(Debug, Serialize, Default, ToSchema)]
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

/// Default log limit for segmented log retrieval
const DEFAULT_LOG_LIMIT: usize = 4096;
/// Default log offset for segmented log retrieval
const DEFAULT_LOG_OFFSET: u64 = 1;

/// Response structure for task status queries
#[derive(Debug, Serialize, Default, ToSchema)]
pub struct TaskStatus {
    status: TaskStatusEnum,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// Shared application state containing worker connections, database, and active builds
#[derive(Clone)]
pub struct AppState {
    pub scheduler: TaskScheduler,
    pub conn: DatabaseConnection,
}

impl AppState {
    /// Create new AppState instance
    pub fn new(
        conn: DatabaseConnection,
        queue_config: Option<crate::scheduler::TaskQueueConfig>,
    ) -> Self {
        let workers = Arc::new(DashMap::new());
        let active_builds = Arc::new(DashMap::new());
        let scheduler = TaskScheduler::new(conn.clone(), workers, active_builds, queue_config);

        Self { scheduler, conn }
    }
}

/// Creates and configures all API routes
pub fn routers() -> Router<AppState> {
    Router::new()
        .route("/ws", any(ws_handler))
        .route("/task", axum::routing::post(task_handler))
        .route("/task-status/{id}", get(task_status_handler))
        .route("/task-output/{id}", get(task_output_handler))
        .route(
            "/task-history-output/{id}",
            get(task_history_output_handler),
        )
        .route("/mr-task/{mr}", get(task_query_by_mr))
        .route("/tasks/{mr}", get(tasks_handler))
        .route("/queue-stats", get(queue_stats_handler))
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

#[utoipa::path(
    get,
    path = "/task-status/{id}",
    params(
        ("id" = String, Path, description = "Task id")
    ),
    responses(
        (status = 200, description = "Task status", body = TaskStatus)
    )
)]
/// Retrieves the current status of a build task by its ID
/// Returns status information including exit code and current state
pub async fn task_status_handler(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let (code, status) = if state.scheduler.active_builds.contains_key(&id) {
        // Task is currently active/building
        (
            StatusCode::OK,
            TaskStatus {
                status: TaskStatusEnum::Building,
                ..Default::default()
            },
        )
    } else {
        // Check database for completed/historical tasks
        match Uuid::parse_str(&id) {
            Ok(id_uuid) => {
                let output = builds::Model::get_by_build_id(id_uuid, &state.conn).await;
                match output {
                    Some(model) => {
                        // Determine task status based on database fields
                        let status = if model.end_at.is_none() {
                            // Not in active_builds and end_at is None => still queued (pending)
                            TaskStatusEnum::Pending
                        } else if model.exit_code.is_none() {
                            TaskStatusEnum::Interrupted
                        } else if model.exit_code.unwrap() == 0 {
                            TaskStatusEnum::Completed
                        } else {
                            TaskStatusEnum::Failed
                        };
                        (
                            StatusCode::OK,
                            TaskStatus {
                                status,
                                exit_code: model.exit_code,
                                ..Default::default()
                            },
                        )
                    }
                    None => (
                        StatusCode::NOT_FOUND,
                        TaskStatus {
                            status: TaskStatusEnum::NotFound,
                            message: Some("Build task not found".to_string()),
                            ..Default::default()
                        },
                    ),
                }
            }
            Err(_) => (
                StatusCode::BAD_REQUEST,
                TaskStatus {
                    status: TaskStatusEnum::NotFound,
                    message: Some("Invalid build id".to_string()),
                    ..Default::default()
                },
            ),
        }
    };
    (code, Json(status))
}

/// Streams build output logs in real-time using Server-Sent Events (SSE)
/// Continuously monitors the log file and streams new content as it becomes available
#[utoipa::path(
    get,
    path = "/task-output/{id}",
    params(
        ("id" = String, Path, description = "Task ID for which to stream output logs")
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
    let log_path_str = format!("{}/{}", get_build_log_dir(), id);
    let log_path = std::path::Path::new(&log_path_str);

    // Return error message if log file doesn't exist
    if !log_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    let file = tokio::fs::File::open(log_path).await.unwrap();
    let mut reader = tokio::io::BufReader::new(file);
    reader.seek(tokio::io::SeekFrom::End(0)).await.unwrap();

    // Build an asynchronous data channel
    // Spawn background task: handle both log + heartbeat with select
    let (tx, rx) = mpsc::unbounded_channel::<Event>();
    tokio::spawn(async move {
        let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
        loop {
            let mut buf = String::new();
            let active_builds = state.scheduler.active_builds.clone();
            let is_building = active_builds.contains_key(&id);

            tokio::select! {
                size = reader.read_to_string(&mut buf) => {
                    let size = size.unwrap();
                    if size == 0 {
                        if is_building {
                            continue;
                        } else {
                            break;
                        }
                    } else {
                        let _ = tx.send(Event::default().data(buf.trim_end()));
                    }
                }
                _ = heartbeat.tick() => {
                    let _ = tx.send(Event::default().comment("heartbeat"));
                }
            }
        }
    });

    let stream = UnboundedReceiverStream::new(rx).map(Ok::<_, Infallible>);

    Ok(Sse::new(stream).keep_alive(KeepAlive::new()))
}

/// Provides the ability to read historical task logs
/// supporting either retrieving the entire log at once or segmenting it by line count.
#[utoipa::path(
    get,
    path = "/task-history-output/{id}",
    params(
        ("id" = String, Path, description = "Task ID whose log to read"),
        ("type" = String, Query, description = "The type of log retrieval: \"full\" indicates full retrieval, while \"segment\" indicates retrieval of segments.",example = "full"),
        ("offset" = Option<u64>, Query, description = "Start line number (1-based)"),
        ("limit"  = Option<usize>, Query, description = "Max number of lines to return"),
    ),
    responses(
        (status = 200, description = "History Log"),
        (status = 400, description = "Invalid parameters"),
        (status = 404, description = "Log file not found"),
        (status = 500, description = "Failed to operate log file"),
    )
)]
pub async fn task_history_output_handler(
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let log_path_str = format!("{}/{}", get_build_log_dir(), id);
    let log_path = std::path::Path::new(&log_path_str);

    // Return error message if log file doesn't exist
    if !log_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "message": "Error: Log File Not Found"
            })),
        );
    }

    let log_type = params.get("type").map(|s| s.as_str());
    match log_type {
        Some("full") => {
            // Read the entire log file
            let file = match tokio::fs::File::open(log_path).await {
                Ok(f) => f,
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "message": "Failed to open log file" })),
                    );
                }
            };
            let mut reader = tokio::io::BufReader::new(file);
            let mut buf = String::new();
            if reader.read_to_string(&mut buf).await.is_err() {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "message": "Failed to read log file" })),
                );
            }

            (StatusCode::OK, Json(serde_json::json!({ "data": buf })))
        }
        Some("segment") => {
            // Parse offset
            let offset = params
                .get("offset")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(DEFAULT_LOG_OFFSET)
                .saturating_sub(1);
            let limit = params
                .get("limit")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(DEFAULT_LOG_LIMIT);

            // Read Range Log
            let file = match tokio::fs::File::open(log_path).await {
                Ok(f) => f,
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "message": "Failed to open log file" })),
                    );
                }
            };
            let reader = tokio::io::BufReader::new(file);
            let mut buf = String::new();
            let mut lines = reader.lines();
            let mut idx = 0;
            let mut count = 0;

            while let Ok(Some(line)) = lines.next_line().await {
                if idx >= offset {
                    buf.push_str(&line);
                    buf.push('\n');
                    count += 1;
                    if count >= limit {
                        break;
                    }
                }
                idx += 1;
            }
            (StatusCode::OK, Json(serde_json::json!({ "data": buf })))
        }
        _ => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "message": "Invalid type" })),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/task",
    request_body = BuildRequest,
    responses(
        (status = 200, description = "Task created", body = serde_json::Value),
        (status = 503, description = "Queue is full", body = serde_json::Value)
    )
)]
/// Creates a new build task and either assigns it immediately or queues it for later processing
/// Returns task ID and status information upon successful creation
pub async fn task_handler(
    State(state): State<AppState>,
    Json(req): Json<BuildRequest>,
) -> impl IntoResponse {
    // Download and get buck2 targets first
    // let target = match download_and_get_buck2_targets(&req.buck_hash, &req.buckconfig_hash).await {
    //     Ok(target) => target,
    //     Err(e) => {
    //         tracing::error!("Failed to download buck2 targets: {}", e);
    //         return (
    //             StatusCode::INTERNAL_SERVER_ERROR,
    //             Json(serde_json::json!({ "message": format!("Failed to download buck2 targets: {}", e) })),
    //         ).into_response();
    //     }
    // };
    // for now we do not extract from file, just use the fixed build target.
    let target = "//...".to_string();

    // Check if there are idle workers available
    if state.scheduler.has_idle_workers() {
        // Have idle workers, directly dispatch task (keep original logic)
        handle_immediate_task_dispatch(state, req, target).await
    } else {
        // No idle workers, add task to queue
        match state
            .scheduler
            .enqueue_task(req.clone(), target.clone())
            .await
        {
            Ok(task_id) => {
                tracing::info!("Task {} queued for later processing", task_id);

                // Save to database (mark as Pending status)
                let model = builds::ActiveModel {
                    build_id: Set(task_id),
                    output_file: Set(format!("{}/{}", get_build_log_dir(), task_id)),
                    exit_code: Set(None),
                    start_at: Set(chrono::Utc::now().naive_utc()),
                    end_at: Set(None),
                    repo_name: Set(req.repo.clone()),
                    target: Set(target.clone()),
                    arguments: Set(req.args.clone().unwrap_or_default().join(" ")),
                    mr: Set(req.mr.clone().unwrap_or_default()),
                };

                if let Err(e) = model.insert(&state.conn).await {
                    tracing::error!("Failed to insert queued task into DB: {}", e);
                }

                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "task_id": task_id.to_string(),
                        "status": "queued",
                        "message": "Task queued for processing when workers become available"
                    })),
                )
                    .into_response()
            }
            Err(e) => {
                tracing::warn!("Failed to queue task: {}", e);
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({
                        "message": format!("Unable to queue task: {}", e)
                    })),
                )
                    .into_response()
            }
        }
    }
}

/// Handle immediate task dispatch logic (original task_handler logic)
async fn handle_immediate_task_dispatch(
    state: AppState,
    req: BuildRequest,
    target: String,
) -> axum::response::Response {
    // Find all idle workers
    let idle_workers = state.scheduler.get_idle_workers();

    // Return error if no workers are available (this shouldn't happen theoretically since we already checked)
    if idle_workers.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"message": "No available workers at the moment"})),
        )
            .into_response();
    }

    // Randomly select an idle worker
    let chosen_index = {
        let mut rng = rand::rng();
        rng.random_range(0..idle_workers.len())
    };
    let chosen_id = idle_workers[chosen_index].clone();
    let task_id = Uuid::now_v7();

    // Create log file for the task
    let log_file = match create_log_file(&task_id.to_string()) {
        Ok(file) => Arc::new(Mutex::new(file)),
        Err(e) => {
            tracing::error!("Failed to create log file for task {}: {}", task_id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"message": "Failed to create log file"})),
            )
                .into_response();
        }
    };

    // Create build information structure
    let build_info = BuildInfo {
        repo: req.repo.clone(),
        target: target.clone(),
        args: req.args.clone(),
        start_at: chrono::Utc::now(),
        mr: req.mr.clone(),
        _worker_id: chosen_id.clone(),
        log_file,
    };

    // Save task to database
    let model = builds::ActiveModel {
        build_id: Set(task_id),
        output_file: Set(format!("{}/{}", get_build_log_dir(), task_id)),
        exit_code: Set(None),
        start_at: Set(build_info.start_at.naive_utc()),
        end_at: Set(None),
        repo_name: Set(build_info.repo.clone()),
        target: Set(build_info.target.clone()),
        arguments: Set(build_info.args.clone().unwrap_or_default().join(" ")),
        mr: Set(build_info.mr.clone().unwrap_or_default()),
    };
    if let Err(e) = model.insert(&state.conn).await {
        tracing::error!("Failed to insert new build task into DB: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"message": "Failed to create task in database"})),
        )
            .into_response();
    }

    // Create WebSocket message for the worker
    let msg = WSMessage::Task {
        id: task_id.to_string(),
        repo: req.repo,
        target,
        args: req.args,
        mr: req.mr.unwrap_or_default(),
    };

    // Send task to the selected worker
    if let Some(mut worker) = state.scheduler.workers.get_mut(&chosen_id) {
        if worker.sender.send(msg).is_ok() {
            worker.status = WorkerStatus::Busy(task_id.to_string());
            state
                .scheduler
                .active_builds
                .insert(task_id.to_string(), build_info);
            tracing::info!(
                "Task {} dispatched immediately to worker {}",
                task_id,
                chosen_id
            );
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "task_id": task_id.to_string(),
                    "client_id": chosen_id,
                    "status": "dispatched"
                })),
            )
                .into_response()
        } else {
            tracing::error!(
                "Failed to send task to supposedly idle worker {}. It might have just disconnected.",
                chosen_id
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    serde_json::json!({"message": "Failed to dispatch task to worker. Please try again."}),
                ),
            ).into_response()
        }
    } else {
        tracing::error!(
            "Chosen idle worker {} not found in map. This should not happen.",
            chosen_id
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"message": "Internal scheduler error."})),
        )
            .into_response()
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
                if let WSMessage::Register { id } = ws_msg {
                    tracing::info!("Worker from {who} registered as: {id}");
                    state.scheduler.workers.insert(
                        id.clone(),
                        WorkerInfo {
                            sender: tx.clone(),
                            status: WorkerStatus::Idle,
                            last_heartbeat: chrono::Utc::now(),
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
                    }
                }
                WSMessage::BuildOutput { id, output } => {
                    // Write build output to the associated log file
                    if let Some(build_info) = state.scheduler.active_builds.get(&id) {
                        let log_file = build_info.log_file.clone();
                        tokio::spawn(async move {
                            let mut file = log_file.lock().await;
                            if let Err(e) = writeln!(file, "{output}") {
                                tracing::error!(
                                    "Failed to write to log file for task {}: {}",
                                    id,
                                    e
                                );
                            } else if let Err(e) = file.flush() {
                                tracing::error!("Failed to flush log file for task {}: {}", id, e);
                            }
                        });
                    } else {
                        tracing::warn!("Received output for unknown task: {}", id);
                    }
                }
                WSMessage::BuildComplete {
                    id,
                    success: _,
                    exit_code,
                    message: _,
                } => {
                    // Handle build completion
                    tracing::info!(
                        "Build {id} completed by worker {current_worker_id} with exit code: {exit_code:?}"
                    );

                    // Remove from active builds and update database
                    state.scheduler.active_builds.remove(&id);
                    let _ = builds::Entity::update_many()
                        .set(builds::ActiveModel {
                            exit_code: Set(exit_code),
                            end_at: Set(Some(chrono::Utc::now().naive_utc())),
                            ..Default::default()
                        })
                        .filter(builds::Column::BuildId.eq(id.parse::<uuid::Uuid>().unwrap()))
                        .exec(&state.conn)
                        .await;

                    // Mark worker as idle again
                    if let Some(mut worker) = state.scheduler.workers.get_mut(current_worker_id) {
                        worker.status = WorkerStatus::Idle;
                    }

                    // After worker becomes idle, notify to process queued tasks
                    state.scheduler.notify_task_available();
                }
                _ => {}
            }
        }
        Message::Close(_) => {
            tracing::info!("Client {who} sent close message.");
            return ControlFlow::Break(());
        }
        _ => {}
    }
    ControlFlow::Continue(())
}

/// Data transfer object for build information in API responses
#[derive(Debug, Serialize, ToSchema)]
pub struct BuildDTO {
    pub build_id: String,
    pub output_file: String,
    pub exit_code: Option<i32>,
    pub start_at: String,
    pub end_at: Option<String>,
    pub repo_name: String,
    pub target: String,
    pub arguments: String,
    pub mr: String,
}

impl BuildDTO {
    /// Converts a database model to a DTO for API responses
    pub fn from_model(model: builds::Model) -> Self {
        Self {
            build_id: model.build_id.to_string(),
            output_file: model.output_file,
            exit_code: model.exit_code,
            start_at: DateTime::<Utc>::from_naive_utc_and_offset(model.start_at, Utc).to_rfc3339(),
            end_at: model
                .end_at
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc).to_rfc3339()),
            repo_name: model.repo_name,
            target: model.target,
            arguments: model.arguments,
            mr: model.mr,
        }
    }
}

#[utoipa::path(
    get,
    path = "/mr-task/{mr}",
    params(
        ("mr" = String, Path, description = "MR number")
    ),
    responses(
        (status = 200, description = "Builds for MR", body = [BuildDTO]),
        (status = 404, description = "No builds found for the given MR", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    )
)]
/// Retrieves all build tasks associated with a specific merge request
/// Returns a list of builds filtered by MR number
pub async fn task_query_by_mr(
    State(state): State<AppState>,
    Path(mr): Path<String>,
) -> Result<Json<Vec<BuildDTO>>, (StatusCode, Json<serde_json::Value>)> {
    let db = &state.conn;
    match builds::Entity::find()
        .filter(builds::Column::Mr.eq(mr))
        .all(db)
        .await
    {
        Ok(models) if !models.is_empty() => {
            let dtos = models.into_iter().map(BuildDTO::from_model).collect();
            Ok(Json(dtos))
        }
        Ok(_) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "message": "No builds found for the given MR" })),
        )),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "message": "Internal server error" })),
            ))
        }
    }
}

/// Task information including current status
#[derive(Debug, Serialize, ToSchema)]
pub struct TaskInfoDTO {
    pub build_id: String,
    pub output_file: String,
    pub exit_code: Option<i32>,
    pub start_at: String,
    pub end_at: Option<String>,
    pub repo_name: String,
    pub target: String,
    pub arguments: String,
    pub mr: String,
    pub status: TaskStatusEnum,
}

impl TaskInfoDTO {
    fn from_model_with_status(model: builds::Model, status: TaskStatusEnum) -> Self {
        Self {
            build_id: model.build_id.to_string(),
            output_file: model.output_file,
            exit_code: model.exit_code,
            start_at: DateTime::<Utc>::from_naive_utc_and_offset(model.start_at, Utc).to_rfc3339(),
            end_at: model
                .end_at
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc).to_rfc3339()),
            repo_name: model.repo_name,
            target: model.target,
            arguments: model.arguments,
            mr: model.mr,
            status,
        }
    }
}

#[utoipa::path(
    get,
    path = "/tasks/{mr}",
    params(
        ("mr" = String, Path, description = "MR number to filter tasks by")
    ),
    responses(
    (status = 200, description = "All tasks with their current status", body = [TaskInfoDTO]),
    (status = 500, description = "Internal error", body = serde_json::Value)
    )
)]
/// Return all tasks with their current status (combining /mr-task and /task-status logic)
pub async fn tasks_handler(
    State(state): State<AppState>,
    Path(mr): Path<String>,
) -> Result<Json<Vec<TaskInfoDTO>>, (StatusCode, Json<serde_json::Value>)> {
    let db = &state.conn;
    let active_builds = state.scheduler.active_builds.clone();
    match builds::Entity::find()
        .filter(builds::Column::Mr.eq(mr))
        .all(db)
        .await
    {
        Ok(models) => {
            let tasks: Vec<TaskInfoDTO> = models
                .into_iter()
                .map(|m| {
                    let id_str = m.build_id.to_string();
                    let status = if active_builds.contains_key(&id_str) {
                        TaskStatusEnum::Building
                    } else if m.end_at.is_none() {
                        // In queue waiting for a worker assignment
                        TaskStatusEnum::Pending
                    } else if m.exit_code.is_none() {
                        TaskStatusEnum::Interrupted
                    } else if m.exit_code == Some(0) {
                        TaskStatusEnum::Completed
                    } else {
                        TaskStatusEnum::Failed
                    };
                    TaskInfoDTO::from_model_with_status(m, status)
                })
                .collect();
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
