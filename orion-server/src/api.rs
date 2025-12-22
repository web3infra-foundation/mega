use crate::model::{builds, tasks};
use crate::scheduler::{
    self, BuildInfo, BuildRequest, TaskQueueStats, TaskScheduler, WorkerInfo, WorkerStatus,
    create_log_file, get_build_log_dir,
};
use anyhow::Result;
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
use chrono::{FixedOffset, Utc};
use common::model::{CommonPage, PageParams};
use dashmap::DashMap;
use futures_util::{SinkExt, Stream, StreamExt};
use orion::ws::{TaskPhase, WSMessage};
use rand::Rng;
use sea_orm::prelude::DateTimeUtc;
use sea_orm::{ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter as _};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::convert::Infallible;
use std::io::Write;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, BufReader};
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;
use utoipa::ToSchema;
use uuid::Uuid;

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

/// Request structure for creating a task
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TaskRequest {
    pub repo: String,
    pub cl_link: String,
    pub cl: i64,
    pub task_name: Option<String>,
    pub template: Option<Value>,
    pub builds: Vec<scheduler::BuildRequest>,
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
        .route("/task-build-list/{id}", get(task_build_list_handler))
        .route("/task-output/{id}", get(task_output_handler))
        .route(
            "/task-history-output/{id}",
            get(task_history_output_handler),
        )
        .route("/tasks/{cl}", get(tasks_handler))
        .route("/queue-stats", get(queue_stats_handler))
        .route("/orion-clients-info", get(get_orion_clients_info))
        .route(
            "/orion-client-status/{id}",
            get(get_orion_client_status_by_id),
        )
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
        ("id" = String, Path, description = "Build ID whose log to read"),
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

            // Split the content into lines and count them
            let lines: Vec<&str> = buf.lines().collect();
            let len = lines.len();

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "data": lines,
                    "len": len
                })),
            )
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
            let mut lines_vec = Vec::new();
            let mut lines = reader.lines();
            let mut idx = 0;
            let mut count = 0;

            while let Ok(Some(line)) = lines.next_line().await {
                if idx >= offset {
                    lines_vec.push(line);
                    count += 1;
                    if count >= limit {
                        break;
                    }
                }
                idx += 1;
            }

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "data": lines_vec,
                    "len": lines_vec.len()
                })),
            )
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
    request_body = TaskRequest,
    responses(
        (status = 200, description = "Task created", body = serde_json::Value),
        (status = 503, description = "Queue is full", body = serde_json::Value)
    )
)]
/// Creates new build tasks based on the builds count in TaskRequest and either assigns them immediately or queues them for later processing
/// Returns task ID and status information upon successful creation
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
        req.task_name.clone(),
        req.template.clone(),
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
                String::new(),
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

/// Handle immediate task dispatch logic (original task_handler logic)
///
/// # Note
/// The `target` field is deprecated, only remain for compatibility reasons.
async fn handle_immediate_task_dispatch(
    state: AppState,
    task_id: Uuid,
    cl_link: &str,
    repo: &str,
    cl: i64,
    req: BuildRequest,
    target: String,
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

    // Create log file for the task
    let log_file = match create_log_file(&build_id.to_string()) {
        Ok(file) => Arc::new(Mutex::new(file)),
        Err(e) => {
            tracing::error!(
                "Failed to create log file for build {}/{}: {}",
                task_id,
                build_id,
                e
            );
            return BuildResult {
                build_id: "".to_string(),
                status: "error".to_string(),
                message: "Failed to create log file".to_string(),
            };
        }
    };

    // Create build information structure
    let build_info = BuildInfo {
        repo: repo.to_string(),
        args: req.args.clone(),
        changes: req.changes.clone(),
        start_at: chrono::Utc::now(),
        cl: cl.to_string(),
        _worker_id: chosen_id.clone(),
        log_file,
    };

    // Use the model's insert_build method for direct insertion
    if let Err(err) = builds::Model::insert_build(
        build_id,
        task_id,
        repo.to_string(),
        target.clone(),
        req.clone(),
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
        args: req.args.clone(),
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
                    success,
                    exit_code,
                    message,
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
                            end_at: Set(Some(
                                Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
                            )),
                            ..Default::default()
                        })
                        .filter(builds::Column::Id.eq(id.parse::<uuid::Uuid>().unwrap()))
                        .exec(&state.conn)
                        .await;

                    // Mark the worker as idle or error depending on whether the task succeeds.
                    if let Some(mut worker) = state.scheduler.workers.get_mut(current_worker_id) {
                        worker.status = if success {
                            WorkerStatus::Idle
                        } else {
                            WorkerStatus::Error(message)
                        };
                    }

                    // After worker becomes idle, notify to process queued tasks
                    state.scheduler.notify_task_available();
                }
                WSMessage::TaskPhaseUpdate { id, phase } => {
                    tracing::info!("Task phase updated by orion worker {id} with: {phase:?}");

                    if let Some(mut worker) = state.scheduler.workers.get_mut(&id) {
                        worker.status = WorkerStatus::Busy {
                            task_id: id,
                            phase: Some(phase),
                        }
                    }
                }
                _ => {}
            }
        }
        Message::Close(_) => {
            tracing::info!("Client {who} sent close message.");
            if let Some(mut worker) = state.scheduler.workers.get_mut(worker_id.as_ref().unwrap()) {
                worker.status = WorkerStatus::Lost
            }
            return ControlFlow::Break(());
        }
        _ => {}
    }
    ControlFlow::Continue(())
}

/// Data transfer object for build information in API responses
#[derive(Debug, Serialize, ToSchema)]
pub struct BuildDTO {
    pub id: String,
    pub task_id: String,
    pub exit_code: Option<i32>,
    pub start_at: String,
    pub end_at: Option<String>,
    pub repo: String,
    pub target: String,
    pub args: Option<Value>,
    pub output_file: String,
    pub created_at: String,
    pub status: TaskStatusEnum,
    pub cause_by: Option<String>,
}

impl BuildDTO {
    /// Converts a database model to a DTO for API responses
    pub fn from_model(model: builds::Model, status: TaskStatusEnum) -> Self {
        Self {
            id: model.id.to_string(),
            task_id: model.task_id.to_string(),
            exit_code: model.exit_code,
            start_at: model.start_at.with_timezone(&Utc).to_rfc3339(),
            end_at: model.end_at.map(|dt| dt.with_timezone(&Utc).to_rfc3339()),
            repo: model.repo,
            target: model.target,
            args: model.args.map(|v| json!(v)),
            output_file: model.output_file,
            created_at: model.created_at.with_timezone(&Utc).to_rfc3339(),
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
        } else if model.exit_code.unwrap() == 0 {
            TaskStatusEnum::Completed
        } else {
            TaskStatusEnum::Failed
        }
    }
}

/// Task information including current status
#[derive(Debug, Serialize, ToSchema)]
pub struct TaskInfoDTO {
    pub task_id: String,
    pub cl_id: i64,
    pub task_name: Option<String>,
    pub template: Option<serde_json::Value>,
    pub created_at: String,
    pub build_list: Vec<BuildDTO>,
}

impl TaskInfoDTO {
    fn from_model(model: tasks::Model, build_list: Vec<BuildDTO>) -> Self {
        Self {
            task_id: model.id.to_string(),
            cl_id: model.cl_id,
            task_name: model.task_name,
            template: model.template,
            created_at: model.created_at.with_timezone(&Utc).to_rfc3339(),
            build_list,
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
                // Query builds associated with this task
                let build_models = builds::Entity::find()
                    .filter(builds::Column::TaskId.eq(m.id))
                    .all(db)
                    .await
                    .unwrap_or_else(|_| vec![]);

                // Convert build models to DTOs with individual status
                let mut build_list: Vec<BuildDTO> = Vec::new();
                for build_model in build_models {
                    let build_id_str = build_model.id.to_string();
                    let is_active = active_builds.contains_key(&build_id_str);
                    let status = BuildDTO::determine_status(&build_model, is_active);
                    let mut dto = BuildDTO::from_model(build_model, status);
                    dto.cause_by = find_caused_by_next_line(&dto.id).await.unwrap_or_else(|e| {
                        tracing::error!("Failed to read cause by line: {}", e);
                        None
                    });
                    build_list.push(dto);
                }

                tasks.push(TaskInfoDTO::from_model(m, build_list));
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

async fn find_caused_by_next_line(id: &str) -> Result<Option<String>> {
    let log_path_str = format!("{}/{}", get_build_log_dir(), id);
    let log_path = std::path::Path::new(&log_path_str);
    tracing::debug!("log path str: {}", log_path_str);

    // Return Ok(None) if log file doesn't exist
    if !log_path.exists() {
        return Ok(None);
    }

    let file = tokio::fs::File::open(log_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to open log file {}: {}", log_path_str, e))?;

    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut last_was_caused = false;

    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read log file: {}", e))?
    {
        if last_was_caused {
            return Ok(Some(line));
        }

        if line.trim() == "Caused by:" {
            last_was_caused = true;
        }
    }
    Ok(None)
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

/// Additional query parameters for querying Orion clients.
/// When no extra conditions are required, this struct can be left empty.
#[derive(Debug, Deserialize, ToSchema, Clone)]
pub struct OrionClientQuery {
    hostname: Option<String>,
}

#[utoipa::path(
    post,
    path = "/orion-clients-info",
    request_body = PageParams<OrionClientQuery>,
    responses(
        (status = 200, description = "Paged Orion client information", body = CommonPage<OrionClientInfo>)
    )
)]
async fn get_orion_clients_info(
    State(state): State<AppState>,
    Json(params): Json<PageParams<OrionClientQuery>>,
) -> Result<Json<CommonPage<OrionClientInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let pagination = params.pagination;
    let query = params.additional.clone();

    let page = pagination.page.max(1);
    // per_page is must in [1, 100]
    let per_page = pagination.per_page.clamp(1u64, 100);
    let offset = (page - 1) * per_page;

    let filtered_items: Vec<OrionClientInfo> = state
        .scheduler
        .workers
        .iter()
        .filter(|entry| {
            if let Some(ref hostname) = query.hostname {
                entry.value().hostname.contains(hostname)
            } else {
                true
            }
        })
        .map(|entry| OrionClientInfo::from_worker(entry.key().clone(), entry.value()))
        .collect();

    let total = filtered_items.len() as u64;

    let items = filtered_items
        .into_iter()
        .skip(offset as usize)
        .take(per_page as usize)
        .collect();

    Ok(Json(CommonPage { total, items }))
}

// Orion client status
#[derive(Debug, Serialize, ToSchema)]
pub struct OrionClientStatus {
    /// Core（Idle / Busy / Error / Lost）
    pub core_status: CoreWorkerStatus,
    /// Only when building
    pub phase: Option<TaskPhase>,
    /// Only when error
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub enum CoreWorkerStatus {
    Idle,
    Busy,
    Error,
    Lost,
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

/// Get Orion client status
#[utoipa::path(
    get,
    path = "/orion-client-status/{id}",
    params(
        ("id" = String, description = "Orion client Id")
    ),
    responses(
        (status = 200, description = "Orion status", body = OrionClientStatus),
        (status = 500, description = "Internal error", body = serde_json::Value)
    )
)]
async fn get_orion_client_status_by_id(
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
