use std::{collections::HashMap, convert::Infallible, pin::Pin, sync::Arc, time::Duration};

use api_model::{
    buck2::{
        api::{OrionBuildResult, OrionServerResponse, TaskBuildRequest},
        status::Status,
        types::{
            LogErrorResponse, LogLinesResponse, LogReadMode, ProjectRelativePath,
            TargetLogLinesResponse, TargetLogQuery, TaskHistoryQuery,
        },
        ws::WSMessage,
    },
    common::{CommonPage, PageParams},
};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response, Sse, sse::Event},
};
use dashmap::DashMap;
use futures::stream::select;
use futures_util::{Stream, StreamExt};
use rand::RngExt;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter as _, QueryOrder, QuerySelect};
use serde_json::{Value, json};
use tokio::sync::watch;
use tokio_stream::wrappers::IntervalStream;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    auto_retry::AutoRetryJudger,
    entity::{builds, targets, targets::TargetState, tasks},
    log::log_service::LogService,
    model::{
        dto::{
            BuildDTO, BuildEventDTO, BuildEventState, BuildTargetDTO, MessageResponse,
            OrionClientInfo, OrionClientQuery, OrionClientStatus, OrionTaskDTO, TargetDTO,
            TargetSummaryDTO, TaskInfoDTO,
        },
        task_status::TaskStatusEnum,
    },
    repository::{
        build_targets::BuildTarget, builds::BuildRepository, orion_tasks::OrionTask,
        targets::TargetRepository, tasks::TaskRepository,
    },
    scheduler::{BuildEventPayload, BuildInfo, TaskQueueStats, WorkerStatus},
};

type MessageErrorResponse = (StatusCode, Json<MessageResponse>);
type JsonValueErrorResponse = (StatusCode, Json<Value>);
type LogSseStream = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;

pub async fn task_output(state: &AppState, id: &str) -> Result<Sse<LogSseStream>, StatusCode> {
    if !state.scheduler.active_builds.contains_key(id) {
        return Err(StatusCode::NOT_FOUND);
    }

    let (stop_tx, stop_rx) = watch::channel(true);

    let log_stop_rx = stop_rx.clone();
    let build_id = id.to_string();
    let log_stream = state
        .log_service
        .subscribe_for_build(build_id.clone())
        .map(|log_event| {
            Ok::<Event, Infallible>(Event::default().event("log").data(log_event.line))
        })
        .take_while(move |_| {
            let stop_rx = log_stop_rx.clone();
            async move { *stop_rx.borrow() }
        });

    let heart_stop_rx = stop_rx.clone();
    let heartbeat_stream = IntervalStream::new(tokio::time::interval(Duration::from_secs(15)))
        .map(|_| Ok::<Event, Infallible>(Event::default().comment("heartbeat")))
        .take_while(move |_| {
            let stop_rx_clone = heart_stop_rx.clone();
            async move { *stop_rx_clone.borrow() }
        });

    let stop_tx_clone = stop_tx.clone();
    let state_clone = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if !state_clone.scheduler.active_builds.contains_key(&build_id) {
                let _ = stop_tx_clone.send(false);
                break;
            }
        }
    });

    Ok(Sse::new(Box::pin(select(log_stream, heartbeat_stream))))
}

fn message_response(message: impl Into<String>) -> Json<MessageResponse> {
    Json(MessageResponse {
        message: message.into(),
    })
}

fn message_error(status: StatusCode, message: impl Into<String>) -> MessageErrorResponse {
    (status, message_response(message))
}

fn value_error(status: StatusCode, message: impl Into<String>) -> JsonValueErrorResponse {
    (status, Json(json!({ "message": message.into() })))
}

fn parse_uuid_or_message_error(
    raw_id: &str,
    invalid_message: &str,
) -> Result<Uuid, MessageErrorResponse> {
    raw_id
        .parse::<Uuid>()
        .map_err(|_| message_error(StatusCode::BAD_REQUEST, invalid_message))
}

fn parse_uuid_or_value_error(
    raw_id: &str,
    invalid_message: &str,
) -> Result<Uuid, JsonValueErrorResponse> {
    raw_id
        .parse::<Uuid>()
        .map_err(|_| value_error(StatusCode::BAD_REQUEST, invalid_message))
}

async fn task_exists_by_id(
    conn: &sea_orm::DatabaseConnection,
    task_id: Uuid,
) -> Result<bool, sea_orm::DbErr> {
    callisto::orion_tasks::Entity::find_by_id(task_id)
        .one(conn)
        .await
        .map(|task| task.is_some())
}

pub async fn task_retry(
    state: &AppState,
    id: &str,
) -> Result<Json<MessageResponse>, MessageErrorResponse> {
    let task_uuid = parse_uuid_or_message_error(id, "Invalid task ID format")?;
    let task = callisto::orion_tasks::Entity::find_by_id(task_uuid)
        .one(&state.conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch task {}: {}", id, e);
            message_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?
        .ok_or_else(|| message_error(StatusCode::NOT_FOUND, "Task not found"))?;

    tracing::info!("Task retry requested for task {} (CL: {})", id, task.cl);
    Ok(message_response(format!("Task {} queued for retry", id)))
}

pub async fn task_get(
    state: &AppState,
    cl: &str,
) -> Result<Json<OrionTaskDTO>, JsonValueErrorResponse> {
    let tasks: Vec<callisto::orion_tasks::Model> = callisto::orion_tasks::Entity::find()
        .filter(callisto::orion_tasks::Column::Cl.eq(cl))
        .all(&state.conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch tasks by CL {}: {}", cl, e);
            value_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?;

    match tasks.len() {
        0 => Err(value_error(StatusCode::NOT_FOUND, "Not found task")),
        1 => Ok(Json(OrionTaskDTO::from(&tasks[0]))),
        _ => Err(value_error(StatusCode::BAD_REQUEST, "Multiple tasks")),
    }
}

pub async fn build_event_get(
    state: &AppState,
    task_id: &str,
) -> Result<Json<Vec<BuildEventDTO>>, JsonValueErrorResponse> {
    let task_uuid = parse_uuid_or_value_error(task_id, "Invalid task ID")?;
    let task_exists = task_exists_by_id(&state.conn, task_uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify task existence {}: {}", task_id, e);
            value_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?;
    if !task_exists {
        return Err(value_error(StatusCode::NOT_FOUND, "Task not found"));
    }

    let build_events = callisto::build_events::Entity::find()
        .filter(callisto::build_events::Column::TaskId.eq(task_uuid))
        .all(&state.conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch build events for task {}: {}", task_id, e);
            value_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?;

    Ok(Json(
        build_events
            .into_iter()
            .map(|m| BuildEventDTO::from(&m))
            .collect(),
    ))
}

pub async fn targets_get(
    state: &AppState,
    task_id: &str,
) -> Result<Json<Vec<BuildTargetDTO>>, MessageErrorResponse> {
    let task_uuid = parse_uuid_or_message_error(task_id, "Invalid task ID")?;
    let task_exists = task_exists_by_id(&state.conn, task_uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify task existence {}: {}", task_id, e);
            message_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?;
    if !task_exists {
        return Err(message_error(StatusCode::NOT_FOUND, "Task not found"));
    }

    let build_targets = callisto::build_targets::Entity::find()
        .filter(callisto::build_targets::Column::TaskId.eq(task_uuid))
        .all(&state.conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch build targets for task {}: {}", task_id, e);
            message_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?;

    Ok(Json(
        build_targets
            .into_iter()
            .map(|build_target| BuildTargetDTO {
                id: build_target.id.to_string(),
                task_id: build_target.task_id.to_string(),
                path: build_target.path,
                latest_state: build_target.latest_state,
            })
            .collect(),
    ))
}

pub async fn build_logs(
    state: &AppState,
    build_id: &str,
) -> Result<Json<LogLinesResponse>, (StatusCode, Json<LogErrorResponse>)> {
    let build_uuid = build_id.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(LogErrorResponse {
                message: "Invalid build ID".to_string(),
            }),
        )
    })?;

    let build_event = callisto::build_events::Entity::find_by_id(build_uuid)
        .one(&state.conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch build event {}: {}", build_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LogErrorResponse {
                    message: "Database error".to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(LogErrorResponse {
                    message: "Build event not found".to_string(),
                }),
            )
        })?;

    let orion_task = callisto::orion_tasks::Entity::find_by_id(build_event.task_id)
        .one(&state.conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch Orion task {}: {}", build_event.task_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LogErrorResponse {
                    message: "Database error".to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(LogErrorResponse {
                    message: "Task not found".to_string(),
                }),
            )
        })?;

    let task_id = build_event.task_id.to_string();
    let repo_name = &orion_task.repo_name;
    let log_content = state
        .log_service
        .read_full_log(&task_id, repo_name, build_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to read log for build {}: {}", build_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LogErrorResponse {
                    message: "Failed to read log".to_string(),
                }),
            )
        })?;

    let lines: Vec<String> = log_content.lines().map(str::to_string).collect();
    Ok(Json(LogLinesResponse {
        len: lines.len(),
        data: lines,
    }))
}

pub async fn build_state(
    state: &AppState,
    build_id: &str,
) -> Result<Json<BuildEventState>, MessageErrorResponse> {
    let build_uuid = parse_uuid_or_message_error(build_id, "Invalid build ID")?;
    if state.scheduler.active_builds.contains_key(build_id) {
        return Ok(Json(BuildEventState::Running));
    }

    let build_event = callisto::build_events::Entity::find_by_id(build_uuid)
        .one(&state.conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch build event {}: {}", build_id, e);
            message_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?
        .ok_or_else(|| message_error(StatusCode::NOT_FOUND, "Build not found"))?;

    let state_enum = match (build_event.end_at, build_event.exit_code) {
        (None, _) => BuildEventState::Running,
        (Some(_), Some(0)) => BuildEventState::Success,
        (Some(_), Some(_)) => BuildEventState::Failure,
        (Some(_), None) => BuildEventState::Failure,
    };
    Ok(Json(state_enum))
}

pub async fn latest_build_result(
    state: &AppState,
    task_id: &str,
) -> Result<Json<BuildEventState>, MessageErrorResponse> {
    let task_uuid = parse_uuid_or_message_error(task_id, "Invalid task ID")?;
    let task_exists = task_exists_by_id(&state.conn, task_uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify task existence {}: {}", task_id, e);
            message_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?;
    if !task_exists {
        return Err(message_error(StatusCode::NOT_FOUND, "Task not found"));
    }

    let latest_build_event = callisto::build_events::Entity::find()
        .filter(callisto::build_events::Column::TaskId.eq(task_uuid))
        .order_by_desc(callisto::build_events::Column::StartAt)
        .one(&state.conn)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to fetch latest build event for task {}: {}",
                task_id,
                e
            );
            message_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?;

    let build_event = latest_build_event.ok_or_else(|| {
        message_error(StatusCode::NOT_FOUND, "No build events found for this task")
    })?;

    let state_enum = match (build_event.end_at, build_event.exit_code) {
        (None, _) => BuildEventState::Running,
        (Some(_), Some(0)) => BuildEventState::Success,
        (Some(_), Some(_)) => BuildEventState::Failure,
        (Some(_), None) => BuildEventState::Failure,
    };
    Ok(Json(state_enum))
}

pub async fn queue_stats(state: &AppState) -> (StatusCode, Json<TaskQueueStats>) {
    let stats = state.scheduler.get_queue_stats().await;
    (StatusCode::OK, Json(stats))
}

pub async fn health_check(state: &AppState) -> (StatusCode, Json<Value>) {
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

pub async fn task_history_output(
    state: &AppState,
    params: &TaskHistoryQuery,
) -> Result<Json<LogLinesResponse>, (StatusCode, Json<LogErrorResponse>)> {
    let log_result = if matches!((params.start, params.end), (None, None)) {
        state
            .log_service
            .read_full_log(&params.task_id, &params.repo, &params.build_id)
            .await
    } else {
        let start = params.start.unwrap_or(0);
        let end = params.end.unwrap_or(usize::MAX);
        state
            .log_service
            .read_log_range(&params.task_id, &params.repo, &params.build_id, start, end)
            .await
    };

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

    let lines: Vec<String> = log_content.lines().map(str::to_string).collect();
    Ok(Json(LogLinesResponse {
        len: lines.len(),
        data: lines,
    }))
}

pub async fn target_logs(
    state: &AppState,
    target_id: &str,
    params: &TargetLogQuery,
) -> Result<Json<TargetLogLinesResponse>, (StatusCode, Json<LogErrorResponse>)> {
    let target_uuid = target_id.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(LogErrorResponse {
                message: "Invalid target id".to_string(),
            }),
        )
    })?;

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

    let build_model = if let Some(build_id) = params.build_id.as_ref() {
        let build_uuid = build_id.parse::<Uuid>().map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(LogErrorResponse {
                    message: "Invalid build id".to_string(),
                }),
            )
        })?;

        match builds::Entity::find_by_id(build_uuid)
            .filter(builds::Column::TargetId.eq(target_uuid))
            .one(&state.conn)
            .await
        {
            Ok(Some(build)) => build,
            Ok(None) => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(LogErrorResponse {
                        message: "Build not found for target".to_string(),
                    }),
                ));
            }
            Err(err) => {
                tracing::error!("Failed to load build {}: {}", build_uuid, err);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(LogErrorResponse {
                        message: "Failed to load build".to_string(),
                    }),
                ));
            }
        }
    } else {
        match builds::Entity::find()
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
            Ok(Json(TargetLogLinesResponse {
                len: lines.len(),
                data: lines,
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
        let target_builds = target_build_map.remove(&target.id).unwrap_or_default();
        target_list.push(crate::entity::targets::TargetWithBuilds::from_model(
            target,
            target_builds,
        ));
    }

    TaskInfoDTO {
        task_id: task.id.to_string(),
        cl_id: task.cl_id,
        task_name: task.task_name,
        template: task.template,
        created_at: task.created_at.with_timezone(&chrono::Utc).to_rfc3339(),
        build_list,
        targets: target_list,
    }
}

pub async fn tasks_by_cl(
    state: &AppState,
    cl: i64,
) -> Result<Json<Vec<TaskInfoDTO>>, (StatusCode, Json<Value>)> {
    let active_builds = state.scheduler.active_builds.clone();
    let task_models = tasks::Entity::find()
        .filter(tasks::Column::ClId.eq(cl))
        .all(&state.conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch tasks: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message": "Failed to fetch tasks"})),
            )
        })?;
    let mut result = Vec::with_capacity(task_models.len());
    for model in task_models {
        result.push(assemble_task_info(model, state, &active_builds).await);
    }
    Ok(Json(result))
}

pub async fn task_targets(
    state: &AppState,
    task_id: &str,
) -> Result<Json<TaskInfoDTO>, (StatusCode, Json<Value>)> {
    let task_uuid = task_id.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"message": "Invalid task ID"})),
        )
    })?;
    let task_model = tasks::Entity::find_by_id(task_uuid)
        .one(&state.conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch task {}: {}", task_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message": "Failed to fetch task"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"message": "Task not found"})),
            )
        })?;
    let info = assemble_task_info(task_model, state, &state.scheduler.active_builds).await;
    Ok(Json(info))
}

pub async fn task_targets_summary(
    state: &AppState,
    task_id: &str,
) -> Result<Json<TargetSummaryDTO>, (StatusCode, Json<Value>)> {
    let task_uuid = task_id.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid task ID" })),
        )
    })?;
    let target_models = targets::Entity::find()
        .filter(targets::Column::TaskId.eq(task_uuid))
        .all(&state.conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch target summary: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to fetch target summary" })),
            )
        })?;
    let mut summary = TargetSummaryDTO {
        task_id: task_id.to_string(),
        pending: 0,
        building: 0,
        completed: 0,
        failed: 0,
        interrupted: 0,
        uninitialized: 0,
    };
    for target in target_models {
        match target.state {
            TargetState::Pending => summary.pending += 1,
            TargetState::Building => summary.building += 1,
            TargetState::Completed => summary.completed += 1,
            TargetState::Failed => summary.failed += 1,
            TargetState::Interrupted => summary.interrupted += 1,
            TargetState::Uninitialized => summary.uninitialized += 1,
        }
    }
    Ok(Json(summary))
}

async fn immediate_retry_work(
    state: &AppState,
    build_id: Uuid,
    idle_workers: &[String],
    build: &builds::Model,
    target: &targets::Model,
    req: &api_model::buck2::api::RetryBuildRequest,
    retry_count: i32,
) -> bool {
    let chosen_index = {
        let mut rng = rand::rng();
        rng.random_range(0..idle_workers.len())
    };
    let chosen_id = idle_workers[chosen_index].clone();
    let start_at = chrono::Utc::now();
    let event = BuildEventPayload::new(
        build.id,
        build.task_id,
        req.cl_link.clone(),
        build.repo.clone(),
        retry_count,
    );
    let build_info = BuildInfo {
        event_payload: event,
        target_id: target.id,
        target_path: target.target_path.clone(),
        changes: req.changes.clone(),
        worker_id: chosen_id.clone(),
        auto_retry_judger: AutoRetryJudger::new(),
        started_at: start_at,
    };
    let msg = api_model::buck2::ws::WSMessage::TaskBuild {
        build_id: build.id.to_string(),
        repo: build.repo.to_string(),
        changes: req.changes.clone(),
        cl_link: req.cl_link.to_string(),
    };
    if let Some(mut worker) = state.scheduler.workers.get_mut(&chosen_id)
        && worker.sender.send(msg).is_ok()
    {
        worker.status = WorkerStatus::Busy {
            build_id: build_id.to_string(),
            phase: None,
        };
        if let Err(e) = TargetRepository::update_state(
            &state.conn,
            target.id,
            TargetState::Building,
            Some(start_at.with_timezone(
                &chrono::FixedOffset::east_opt(0).unwrap_or_else(|| unreachable!()),
            )),
            None,
            None,
        )
        .await
        {
            tracing::error!("Failed to update target state to Building: {}", e);
        }
        state
            .scheduler
            .active_builds
            .insert(build.id.to_string(), build_info);
        true
    } else {
        false
    }
}

pub async fn build_retry(
    state: &AppState,
    req: api_model::buck2::api::RetryBuildRequest,
) -> Response {
    let build_id = match req.build_id.parse::<uuid::Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"message": "Invalid build ID format"})),
            )
                .into_response();
        }
    };
    if state.scheduler.active_builds.contains_key(&req.build_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"message": "The build already exists"})),
        )
            .into_response();
    }
    let build = match builds::Entity::find_by_id(build_id).one(&state.conn).await {
        Ok(Some(build)) => build,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"message": "Build not found"})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message": "Database find failed"})),
            )
                .into_response();
        }
    };
    let retry_count = build.retry_count + 1;
    let target_model = match targets::Entity::find_by_id(build.target_id)
        .one(&state.conn)
        .await
    {
        Ok(Some(target)) => target,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"message": "Target not found for build"})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message": "Database find failed"})),
            )
                .into_response();
        }
    };

    let idle_workers = state.scheduler.get_idle_workers();
    if idle_workers.is_empty() {
        let new_build_id = Uuid::now_v7();
        match state
            .scheduler
            .enqueue_task_with_build_id(
                new_build_id,
                build.task_id,
                &req.cl_link,
                build.repo.clone(),
                req.changes.clone(),
                target_model.target_path.clone(),
                retry_count,
            )
            .await
        {
            Ok(()) => (
                StatusCode::OK,
                Json(json!({"message": "Build queued for later processing"})),
            )
                .into_response(),
            Err(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"message": "No available workers at the moment"})),
            )
                .into_response(),
        }
    } else if immediate_retry_work(
        state,
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
            Json(json!({"message": "Build retry dispatched immediately to worker"})),
        )
            .into_response()
    } else {
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({"message": "Failed to dispatch build retry to worker"})),
        )
            .into_response()
    }
}

async fn activate_worker(
    build_info: &BuildInfo,
    scheduler: &crate::scheduler::TaskScheduler,
) -> OrionBuildResult {
    let msg = WSMessage::TaskBuild {
        build_id: build_info.event_payload.build_event_id.to_string(),
        repo: build_info.event_payload.repo.clone(),
        changes: build_info.changes.clone(),
        cl_link: build_info.event_payload.cl_link.clone(),
    };
    if let Some(worker) = scheduler.workers.get_mut(&build_info.worker_id)
        && worker.sender.send(msg).is_ok()
    {
        scheduler.active_builds.insert(
            build_info.event_payload.build_event_id.to_string(),
            build_info.clone(),
        );
        return OrionBuildResult {
            build_id: build_info.event_payload.build_event_id.to_string(),
            status: "dispatched".to_string(),
            message: format!("Build dispatched to worker {}", build_info.worker_id),
        };
    }
    scheduler.release_worker(&build_info.worker_id).await;
    OrionBuildResult {
        build_id: build_info.event_payload.build_event_id.to_string(),
        status: "error".to_string(),
        message: "Failed to dispatch task to worker".to_string(),
    }
}

async fn handle_immediate_task_dispatch_v2(
    state: &AppState,
    task_id: Uuid,
    repo: &str,
    cl_link: &str,
    changes: Vec<Status<ProjectRelativePath>>,
) -> OrionBuildResult {
    let build_event_id = Uuid::now_v7();
    let Some(chosen_id) = state
        .scheduler
        .search_and_claim_worker(&build_event_id.to_string())
    else {
        return OrionBuildResult {
            build_id: "".to_string(),
            status: "error".to_string(),
            message: "No available workers at the moment".to_string(),
        };
    };

    let target_id = Uuid::now_v7();
    let target_path =
        match BuildTarget::insert_default_target(target_id, task_id, &state.conn).await {
            Ok(default_path) => default_path,
            Err(_) => {
                state.scheduler.release_worker(&chosen_id).await;
                return OrionBuildResult {
                    build_id: "".to_string(),
                    status: "error".to_string(),
                    message: format!("Failed to prepare target for task {}", task_id),
                };
            }
        };

    let event = BuildEventPayload::new(
        build_event_id,
        task_id,
        cl_link.to_string(),
        repo.to_string(),
        0,
    );
    let build_info = BuildInfo {
        event_payload: event.clone(),
        changes: changes.clone(),
        target_id,
        target_path,
        worker_id: chosen_id,
        auto_retry_judger: AutoRetryJudger::new(),
        started_at: chrono::Utc::now(),
    };

    if let Err(e) = callisto::build_events::Model::insert_build(
        build_event_id,
        task_id,
        repo.to_string(),
        &state.conn,
    )
    .await
    {
        state.scheduler.release_worker(&build_info.worker_id).await;
        return OrionBuildResult {
            build_id: "".to_string(),
            status: "error".to_string(),
            message: format!(
                "Failed to insert build event into database for task {}: {}",
                task_id, e
            ),
        };
    }
    activate_worker(&build_info, &state.scheduler).await
}

async fn handle_immediate_task_dispatch(
    state: &AppState,
    task_id: Uuid,
    repo: &str,
    cl_link: &str,
    changes: Vec<Status<ProjectRelativePath>>,
) -> OrionBuildResult {
    let idle_workers = state.scheduler.get_idle_workers();
    if idle_workers.is_empty() {
        return OrionBuildResult {
            build_id: "".to_string(),
            status: "error".to_string(),
            message: "No available workers at the moment".to_string(),
        };
    }
    let chosen_id = {
        let mut rng = rand::rng();
        idle_workers[rng.random_range(0..idle_workers.len())].clone()
    };
    let build_id = Uuid::now_v7();
    let target_model = match state.scheduler.ensure_target(task_id, "").await {
        Ok(target) => target,
        Err(_) => {
            return OrionBuildResult {
                build_id: "".to_string(),
                status: "error".to_string(),
                message: "Failed to prepare target ".to_string(),
            };
        }
    };
    let start_at = chrono::Utc::now();
    let _ = TargetRepository::update_state(
        &state.conn,
        target_model.id,
        TargetState::Building,
        Some(
            start_at
                .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap_or_else(|| unreachable!())),
        ),
        None,
        None,
    )
    .await;

    let build_info = BuildInfo {
        event_payload: BuildEventPayload::new(
            build_id,
            task_id,
            cl_link.to_string(),
            repo.to_string(),
            0,
        ),
        changes: changes.clone(),
        target_id: target_model.id,
        target_path: target_model.target_path.clone(),
        worker_id: chosen_id.clone(),
        auto_retry_judger: AutoRetryJudger::new(),
        started_at: start_at,
    };
    if BuildRepository::insert_build(
        build_id,
        task_id,
        target_model.id,
        repo.to_string(),
        &state.conn,
    )
    .await
    .is_err()
    {
        return OrionBuildResult {
            build_id: "".to_string(),
            status: "error".to_string(),
            message: "Failed to insert builds into database".to_string(),
        };
    }
    let msg = WSMessage::TaskBuild {
        build_id: build_id.to_string(),
        repo: repo.to_string(),
        changes,
        cl_link: cl_link.to_string(),
    };
    if let Some(mut worker) = state.scheduler.workers.get_mut(&chosen_id)
        && worker.sender.send(msg).is_ok()
    {
        worker.status = WorkerStatus::Busy {
            build_id: build_id.to_string(),
            phase: None,
        };
        state
            .scheduler
            .active_builds
            .insert(build_id.to_string(), build_info);
        OrionBuildResult {
            build_id: build_id.to_string(),
            status: "dispatched".to_string(),
            message: format!("Build dispatched to worker {}", chosen_id),
        }
    } else {
        OrionBuildResult {
            build_id: "".to_string(),
            status: "error".to_string(),
            message: "Failed to dispatch task to worker".to_string(),
        }
    }
}

pub async fn task_handler_v2(state: &AppState, req: TaskBuildRequest) -> Response {
    let task_id = Uuid::now_v7();
    if let Err(err) =
        OrionTask::insert_task(task_id, &req.cl_link, &req.repo, &req.changes, &state.conn).await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"message": format!("Failed to insert task into database: {}", err)})),
        )
            .into_response();
    }
    let result = if state.scheduler.has_idle_workers() {
        handle_immediate_task_dispatch_v2(state, task_id, &req.repo, &req.cl_link, req.changes)
            .await
    } else {
        match state
            .scheduler
            .enqueue_task_v2(task_id, &req.cl_link, req.repo, req.changes, 0)
            .await
        {
            Ok(build_id) => OrionBuildResult {
                build_id: build_id.to_string(),
                status: "queued".to_string(),
                message: "Task queued for processing when workers become available".to_string(),
            },
            Err(e) => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({"message": format!("Unable to queue task: {}", e)})),
                )
                    .into_response();
            }
        }
    };
    (
        StatusCode::OK,
        Json(OrionServerResponse {
            task_id: task_id.to_string(),
            results: vec![result],
        }),
    )
        .into_response()
}

pub async fn task_handler_v1(state: &AppState, req: TaskBuildRequest) -> Response {
    let task_id = Uuid::now_v7();
    let task_name = format!("CL-{}-{}", req.cl_link, task_id);
    if let Err(err) = TaskRepository::insert_task(
        task_id,
        req.cl_id,
        Some(task_name),
        None,
        chrono::Utc::now().into(),
        &state.conn,
    )
    .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"message": format!("Failed to insert task into database: {}", err)})),
        )
            .into_response();
    }
    let mut results = Vec::new();
    if state.scheduler.has_idle_workers() {
        results.push(
            handle_immediate_task_dispatch(
                state,
                task_id,
                &req.repo,
                &req.cl_link,
                req.changes.clone(),
            )
            .await,
        );
    } else {
        match state
            .scheduler
            .enqueue_task(
                task_id,
                &req.cl_link,
                req.repo.clone(),
                req.changes.clone(),
                None,
                0,
            )
            .await
        {
            Ok(build_id) => results.push(OrionBuildResult {
                build_id: build_id.to_string(),
                status: "queued".to_string(),
                message: "Task queued for processing when workers become available".to_string(),
            }),
            Err(e) => results.push(OrionBuildResult {
                build_id: "".to_string(),
                status: "error".to_string(),
                message: format!("Unable to queue task: {}", e),
            }),
        }
    }
    (
        StatusCode::OK,
        Json(OrionServerResponse {
            task_id: task_id.to_string(),
            results,
        }),
    )
        .into_response()
}

pub async fn task_build_list(state: &AppState, id: &str) -> Response {
    let task_id = match id.parse::<uuid::Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"message": "Invalid task ID format"})),
            )
                .into_response();
        }
    };
    match TaskRepository::get_builds_by_task_id(task_id, &state.conn).await {
        Some(build_ids) => {
            let build_ids_str: Vec<String> =
                build_ids.into_iter().map(|uuid| uuid.to_string()).collect();
            (StatusCode::OK, Json(build_ids_str)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"message": "Task not found"})),
        )
            .into_response(),
    }
}

pub async fn get_orion_clients_info(
    state: &AppState,
    params: PageParams<OrionClientQuery>,
) -> Result<Json<CommonPage<OrionClientInfo>>, (StatusCode, Json<Value>)> {
    let pagination = params.pagination;
    let query = params.additional.clone();
    let page = pagination.page.max(1);
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
                items.push(OrionClientInfo {
                    client_id: entry.key().clone(),
                    hostname: entry.value().hostname.clone(),
                    orion_version: entry.value().orion_version.clone(),
                    start_time: entry.value().start_time,
                    last_heartbeat: entry.value().last_heartbeat,
                });
            }
        }
    }
    Ok(Json(CommonPage { total, items }))
}

pub async fn get_orion_client_status_by_id(
    state: &AppState,
    id: &str,
) -> Result<Json<OrionClientStatus>, (StatusCode, Json<Value>)> {
    let worker = state.scheduler.workers.get(id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({"message": "Orion client not found"})),
        )
    })?;
    Ok(Json(OrionClientStatus::from_worker_status(&worker)))
}
