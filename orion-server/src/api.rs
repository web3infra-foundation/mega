use crate::auto_retry::AutoRetryJudger;
use crate::common::model::{CommonPage, PageParams};
use crate::log::log_service::{LogEvent, LogService};
use crate::model::{builds, tasks};
use crate::scheduler::{
    self, BuildInfo, BuildRequest, TaskQueueStats, TaskScheduler, WorkerInfo, WorkerStatus,
    resolve_target,
};
use anyhow::Result;
use axum::extract::Query;
use axum::routing::post;
use axum::{
    Json, Router,
    extract::{
        ConnectInfo, Path, State, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    http::StatusCode,
    response::{IntoResponse, Sse, sse::Event},
    routing::{any, get},
};
use chrono::{FixedOffset, Utc};
use dashmap::DashMap;
use futures::stream::select;
use futures_util::{SinkExt, Stream, StreamExt};
use orion::ws::{TaskPhase, WSMessage};
use rand::Rng;
use sea_orm::prelude::DateTimeUtc;
use sea_orm::{ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter as _};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::IntervalStream;
use utoipa::ToSchema;
use uuid::Uuid;

const RETRY_COUNT_MAX: u32 = 3;

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
    Canceled,
    Completed,
    #[default]
    NotFound,
}

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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TaskHistoryQuery {
    pub task_id: String,
    pub build_id: String,
    pub repo: String,
    pub start: Option<usize>,
    pub end: Option<usize>,
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
        .route("/tasks/{cl}", get(tasks_handler))
        .route("/queue-stats", get(queue_stats_handler))
        .route("/orion-clients-info", post(get_orion_clients_info))
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
        (status = 200, description = "History Log"),
        (status = 400, description = "Invalid parameters"),
        (status = 404, description = "Log file not found"),
        (status = 500, description = "Failed to operate log file"),
    )
)]
pub async fn task_history_output_handler(
    State(state): State<AppState>,
    Query(params): Query<TaskHistoryQuery>,
) -> impl IntoResponse {
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
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "message": "Failed to read log file"
                })),
            );
        }
    };

    // Split the content into lines and count them
    let lines: Vec<&str> = log_content.lines().collect();
    let len = lines.len();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "data": lines,
            "len": len
        })),
    )
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
) -> BuildResult {
    let (resolved_target, fallback_used) = resolve_target(req.target.clone());
    let target_for_worker = if fallback_used {
        tracing::warn!(
            "Fallback to legacy single-target mode for immediate task {} build",
            task_id
        );
        None
    } else {
        Some(resolved_target.clone())
    };

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

    // Create build information structure
    let build_info = BuildInfo {
        task_id: task_id.to_string(),
        build_id: build_id.to_string(),
        repo: repo.to_string(),
        changes: req.changes.clone(),
        start_at: chrono::Utc::now(),
        cl: cl.to_string(),
        _worker_id: chosen_id.clone(),
        target: target_for_worker.clone(),
        auto_retry_judger: AutoRetryJudger::new(),
        retry_count: 0,
    };

    // Use the model's insert_build method for direct insertion
    if let Err(err) = builds::Model::insert_build(
        build_id,
        task_id,
        repo.to_string(),
        resolved_target.clone(),
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
        target: target_for_worker,
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
                    if let Some(build_info) = state.scheduler.active_builds.get(&id) {
                        let mut build_info = build_info.clone();

                        // Judge auto retry by exit code
                        build_info
                            .auto_retry_judger
                            .judge_by_exit_code(exit_code.unwrap_or(0));

                        let (can_auto_retry, mut retry_count) = (
                            build_info.auto_retry_judger.get_can_auto_retry(),
                            build_info.retry_count,
                        );

                        // Reset auto retry judger
                        build_info.auto_retry_judger = AutoRetryJudger::new();

                        if can_auto_retry && retry_count < RETRY_COUNT_MAX {
                            // Restart this build for retry and add retry time
                            retry_count += 1;

                            // Update build information
                            build_info.retry_count = retry_count;
                            state
                                .scheduler
                                .active_builds
                                .alter(&id, |_, _| build_info.clone());

                            // Update database
                            let _ = builds::Entity::update_many()
                                .set(builds::ActiveModel {
                                    retry_count: Set(retry_count),
                                    ..Default::default()
                                })
                                .filter(builds::Column::Id.eq(id.parse::<uuid::Uuid>().unwrap()))
                                .exec(&state.conn)
                                .await;

                            // Send Task to the same worker
                            let msg = WSMessage::Task {
                                id: build_info.build_id.clone(),
                                repo: build_info.repo,
                                changes: build_info.changes,
                                cl_link: build_info.cl,
                                target: build_info.target.clone(),
                            };
                            let worker_id = build_info._worker_id;
                            if let Some(worker) = state.scheduler.workers.get_mut(&worker_id)
                                && worker.sender.send(msg).is_ok()
                            {
                                tracing::info!(
                                    "Retry build: {}, worker: {}",
                                    build_info.build_id,
                                    worker_id
                                );
                            } else {
                                tracing::error!("Retry build send to worker Failed");

                                // If retry dispatch fails, treat this build as finished and clean up.
                                // Remove from active builds
                                state.scheduler.active_builds.remove(&id);

                                // Update database with final state
                                let _ = builds::Entity::update_many()
                                    .set(builds::ActiveModel {
                                        exit_code: Set(exit_code),
                                        end_at: Set(Some(
                                            Utc::now()
                                                .with_timezone(&FixedOffset::east_opt(0).unwrap()),
                                        )),
                                        retry_count: Set(retry_count),
                                        ..Default::default()
                                    })
                                    .filter(
                                        builds::Column::Id.eq(id.parse::<uuid::Uuid>().unwrap()),
                                    )
                                    .exec(&state.conn)
                                    .await;

                                // Notify scheduler to process queued tasks
                                state.scheduler.notify_task_available();
                            }
                        } else {
                            // Send final log event
                            let log_event = LogEvent {
                                task_id: build_info.task_id.clone(),
                                repo_name: LogService::last_segment(&build_info.repo.clone())
                                    .to_string(),
                                build_id: build_info.build_id.clone(),
                                line: String::from(""),
                                is_end: true,
                            };
                            state.log_service.publish(log_event);

                            // Remove from active builds
                            state.scheduler.active_builds.remove(&id);

                            // Update database
                            let _ = builds::Entity::update_many()
                                .set(builds::ActiveModel {
                                    exit_code: Set(exit_code),
                                    end_at: Set(Some(
                                        Utc::now()
                                            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
                                    )),
                                    retry_count: Set(retry_count),
                                    ..Default::default()
                                })
                                .filter(builds::Column::Id.eq(id.parse::<uuid::Uuid>().unwrap()))
                                .exec(&state.conn)
                                .await;

                            // Mark the worker as idle or error depending on whether the task succeeds.
                            if let Some(mut worker) =
                                state.scheduler.workers.get_mut(current_worker_id)
                            {
                                worker.status = if success {
                                    WorkerStatus::Idle
                                } else {
                                    WorkerStatus::Error(message)
                                };
                            }

                            // After worker becomes idle, notify to process queued tasks
                            state.scheduler.notify_task_available();
                        }
                    } else {
                        tracing::error!("Not found build: {id}");
                    }
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

/// Aggregate CL-level status from per-target builds.
///
/// Returns a tuple of (overall_status, partial_success). Priority:
/// 1) Any Failed/Interrupted/Canceled -> Failed
/// 2) Else any Building/Pending -> Building
/// 3) Else any Completed -> Completed
/// 4) Else -> Pending
///
/// `partial_success` is true if at least one build Completed AND at least one
/// other build is Running/Pending or Failed/Interrupted/Canceled.
fn aggregate_task_status(builds: &[BuildDTO]) -> (TaskStatusEnum, bool) {
    if builds.is_empty() {
        return (TaskStatusEnum::NotFound, false);
    }

    let has_success = builds
        .iter()
        .any(|b| matches!(b.status, TaskStatusEnum::Completed));
    let has_running = builds
        .iter()
        .any(|b| matches!(b.status, TaskStatusEnum::Building | TaskStatusEnum::Pending));
    let has_failure = builds.iter().any(|b| {
        matches!(
            b.status,
            TaskStatusEnum::Failed | TaskStatusEnum::Interrupted | TaskStatusEnum::Canceled
        )
    });

    // Status priority: failure > running/pending > completed > all pending/not found
    let status = match (has_failure, has_running, has_success) {
        (true, _, _) => TaskStatusEnum::Failed,
        (false, true, _) => TaskStatusEnum::Building,
        (false, false, true) => TaskStatusEnum::Completed,
        _ => TaskStatusEnum::Pending,
    };

    // Partial success when at least one succeeded while others are running or failed.
    let partial_success = has_success && (has_failure || has_running);

    (status, partial_success)
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
    pub status: TaskStatusEnum,
    pub partial_success: bool,
}

impl TaskInfoDTO {
    fn from_model(
        model: tasks::Model,
        build_list: Vec<BuildDTO>,
        status: TaskStatusEnum,
        partial_success: bool,
    ) -> Self {
        Self {
            task_id: model.id.to_string(),
            cl_id: model.cl_id,
            task_name: model.task_name,
            template: model.template,
            created_at: model.created_at.with_timezone(&Utc).to_rfc3339(),
            build_list,
            status,
            partial_success,
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

                let mut build_list: Vec<BuildDTO> = Vec::new();
                for build_model in build_models {
                    let build_id_str = build_model.id.to_string();
                    let is_active = active_builds.contains_key(&build_id_str);
                    let status = BuildDTO::determine_status(&build_model, is_active);
                    let mut dto = BuildDTO::from_model(build_model, status);

                    // Read log from LogService instead of file system
                    match state
                        .log_service
                        .read_full_log(
                            &m.id.to_string(),
                            &LogService::last_segment(&dto.repo).to_string(),
                            &build_id_str,
                        )
                        .await
                    {
                        Ok(log_content) => {
                            dto.cause_by = find_caused_by_next_line_in_content(&log_content).await;
                        }
                        Err(e) => {
                            tracing::error!("Failed to read log for build {}: {}", build_id_str, e);
                            dto.cause_by = None;
                        }
                    }

                    build_list.push(dto);
                }

                let (status, partial_success) = aggregate_task_status(&build_list);
                tasks.push(TaskInfoDTO::from_model(
                    m,
                    build_list,
                    status,
                    partial_success,
                ));
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

#[cfg(test)]
mod tests {
    use super::{BuildDTO, TaskStatusEnum, aggregate_task_status};

    fn dummy_build(status: TaskStatusEnum) -> BuildDTO {
        BuildDTO {
            id: "build".to_string(),
            task_id: "task".to_string(),
            exit_code: None,
            start_at: "2024-01-01T00:00:00Z".to_string(),
            end_at: None,
            repo: "repo".to_string(),
            target: "//:target".to_string(),
            args: None,
            output_file: "path.log".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            status,
            cause_by: None,
        }
    }

    #[test]
    fn aggregate_status_handles_partial_success() {
        let builds = vec![
            dummy_build(TaskStatusEnum::Completed),
            dummy_build(TaskStatusEnum::Failed),
        ];

        let (status, partial) = aggregate_task_status(&builds);
        assert!(partial);
        assert!(matches!(status, TaskStatusEnum::Failed));
    }

    #[test]
    fn aggregate_status_handles_running() {
        let builds = vec![
            dummy_build(TaskStatusEnum::Completed),
            dummy_build(TaskStatusEnum::Building),
        ];

        let (status, partial) = aggregate_task_status(&builds);
        assert!(partial);
        assert!(matches!(status, TaskStatusEnum::Building));
    }

    #[test]
    fn aggregate_status_full_success() {
        let builds = vec![
            dummy_build(TaskStatusEnum::Completed),
            dummy_build(TaskStatusEnum::Completed),
        ];

        let (status, partial) = aggregate_task_status(&builds);
        assert!(!partial);
        assert!(matches!(status, TaskStatusEnum::Completed));
    }

    #[test]
    fn aggregate_status_success_and_canceled() {
        let builds = vec![
            dummy_build(TaskStatusEnum::Completed),
            dummy_build(TaskStatusEnum::Canceled),
        ];

        let (status, partial) = aggregate_task_status(&builds);
        assert!(partial);
        assert!(matches!(status, TaskStatusEnum::Failed));
    }

    #[test]
    fn aggregate_status_empty_builds() {
        let builds = vec![];
        let (status, partial) = aggregate_task_status(&builds);
        assert!(!partial);
        assert!(matches!(status, TaskStatusEnum::NotFound));
    }

    #[test]
    fn aggregate_status_all_pending() {
        let builds = vec![
            dummy_build(TaskStatusEnum::Pending),
            dummy_build(TaskStatusEnum::Pending),
        ];
        let (status, partial) = aggregate_task_status(&builds);
        assert!(!partial);
        assert!(matches!(status, TaskStatusEnum::Building));
    }

    #[test]
    fn aggregate_status_all_failed() {
        let builds = vec![
            dummy_build(TaskStatusEnum::Failed),
            dummy_build(TaskStatusEnum::Failed),
        ];
        let (status, partial) = aggregate_task_status(&builds);
        assert!(!partial);
        assert!(matches!(status, TaskStatusEnum::Failed));
    }

    #[test]
    fn aggregate_status_interrupted_and_failed() {
        let builds = vec![
            dummy_build(TaskStatusEnum::Interrupted),
            dummy_build(TaskStatusEnum::Failed),
        ];
        let (status, partial) = aggregate_task_status(&builds);
        assert!(!partial);
        assert!(matches!(status, TaskStatusEnum::Failed));
    }

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
