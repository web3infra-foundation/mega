use std::{net::SocketAddr, ops::ControlFlow};

use api_model::buck2::{types::LogEvent, ws::WSMessage};
use axum::{
    extract::{
        ConnectInfo, State, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter as _};
use tokio::sync::mpsc::{self, UnboundedSender};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    auto_retry::AutoRetryJudger,
    entity::{builds, targets::TargetState},
    log::log_service::LogService,
    repository::{build_events::BuildEvent, targets::TargetRepository},
    scheduler::{WorkerInfo, WorkerStatus},
};

const RETRY_COUNT_MAX: i32 = 3;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("{addr} connected. Waiting for registration...");
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

async fn handle_socket(socket: WebSocket, who: SocketAddr, state: AppState) {
    let (tx, mut rx) = mpsc::unbounded_channel::<WSMessage>();
    let mut worker_id: Option<String> = None;
    let (mut sender, mut receiver) = socket.split();

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let msg_str = serde_json::to_string(&msg).unwrap_or_default();
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
        _ = send_task => {},
        result = recv_task => {
            if let Ok(final_worker_id) = result {
                worker_id = final_worker_id;
            }
        }
    }

    if let Some(id) = &worker_id {
        tracing::info!("Cleaning up for worker: {id} from {who}.");
        state.scheduler.workers.remove(id);
    } else {
        tracing::info!("Cleaning up unregistered connection from {who}.");
    }
    tracing::info!("Websocket context {who} destroyed");
}

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
            let ws_msg = ws_msg.unwrap_or(WSMessage::Heartbeat);

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
                    state.scheduler.notify_task_available();
                } else {
                    tracing::error!(
                        "First message from {who} was not Register. Closing connection."
                    );
                    return ControlFlow::Break(());
                }
                return ControlFlow::Continue(());
            }

            let current_worker_id = worker_id.as_ref().unwrap_or(&String::new()).clone();
            match ws_msg {
                WSMessage::Register { .. } => {
                    tracing::warn!(
                        "Worker {current_worker_id} sent Register message again. Ignoring."
                    );
                }
                WSMessage::Heartbeat => {
                    if let Some(mut worker) = state.scheduler.workers.get_mut(&current_worker_id) {
                        worker.last_heartbeat = chrono::Utc::now();
                        if let WorkerStatus::Error(_) = worker.status {
                            worker.status = WorkerStatus::Idle;
                        }
                    }
                }
                WSMessage::TaskBuildOutput { build_id, output } => {
                    if let Some(build_info) = state.scheduler.active_builds.get(&build_id) {
                        let log_event = LogEvent {
                            task_id: build_info.event_payload.task_id.to_string(),
                            repo_name: LogService::last_segment(&build_info.event_payload.repo)
                                .to_string(),
                            build_id: build_id.clone(),
                            line: output.clone(),
                            is_end: false,
                        };
                        state.log_service.publish(log_event);
                    }
                    if let Some(mut build_info) = state.scheduler.active_builds.get_mut(&build_id) {
                        build_info.auto_retry_judger.judge_by_output(&output);
                    }
                }
                WSMessage::TaskBuildCompleteV2 {
                    build_id,
                    success,
                    exit_code,
                    message,
                }
                | WSMessage::TaskBuildComplete {
                    build_id,
                    success,
                    exit_code,
                    message,
                } => {
                    let (
                        mut auto_retry_judger,
                        mut retry_count,
                        repo,
                        changes,
                        cl_link,
                        task_id,
                        target_id,
                    ) = if let Some(build_info) = state.scheduler.active_builds.get(&build_id) {
                        (
                            build_info.auto_retry_judger.clone(),
                            build_info.event_payload.retry_count,
                            build_info.event_payload.repo.clone(),
                            build_info.changes.clone(),
                            build_info.event_payload.cl_link.clone(),
                            build_info.event_payload.task_id,
                            build_info.target_id,
                        )
                    } else {
                        return ControlFlow::Continue(());
                    };

                    auto_retry_judger.judge_by_exit_code(exit_code.unwrap_or(0));
                    if auto_retry_judger.get_can_auto_retry() && retry_count < RETRY_COUNT_MAX {
                        retry_count += 1;
                        if let Some(mut build_info) =
                            state.scheduler.active_builds.get_mut(&build_id)
                        {
                            build_info.event_payload.retry_count = retry_count;
                            build_info.auto_retry_judger = AutoRetryJudger::new();
                        }
                        let _ = builds::Entity::update_many()
                            .set(builds::ActiveModel {
                                retry_count: Set(retry_count),
                                ..Default::default()
                            })
                            .filter(
                                builds::Column::Id.eq(build_id
                                    .parse::<uuid::Uuid>()
                                    .unwrap_or_else(|_| Uuid::nil())),
                            )
                            .exec(&state.conn)
                            .await;

                        let msg = WSMessage::TaskBuild {
                            build_id: build_id.clone(),
                            repo: repo.clone(),
                            cl_link,
                            changes,
                        };
                        if let Some(worker) = state.scheduler.workers.get_mut(&current_worker_id)
                            && worker.sender.send(msg).is_ok()
                        {
                            return ControlFlow::Continue(());
                        }
                    }

                    state.log_service.publish(LogEvent {
                        task_id: task_id.to_string(),
                        repo_name: LogService::last_segment(&repo).to_string(),
                        build_id: build_id.to_string(),
                        line: String::new(),
                        is_end: true,
                    });
                    state.scheduler.active_builds.remove(&build_id);

                    let _ =
                        BuildEvent::update_build_complete_result(&build_id, exit_code, &state.conn)
                            .await;

                    let target_state = match (success, exit_code) {
                        (true, Some(0)) => TargetState::Completed,
                        (_, None) => TargetState::Interrupted,
                        _ => TargetState::Failed,
                    };
                    let error_summary = if matches!(target_state, TargetState::Failed) {
                        match state
                            .log_service
                            .read_full_log(
                                &task_id.to_string(),
                                &LogService::last_segment(&repo),
                                &build_id.to_string(),
                            )
                            .await
                        {
                            Ok(content) => find_caused_by_next_line_in_content(&content).await,
                            Err(_) => None,
                        }
                    } else {
                        None
                    };
                    let _ = TargetRepository::update_state(
                        &state.conn,
                        target_id,
                        target_state,
                        None,
                        Some(chrono::Utc::now().with_timezone(
                            &chrono::FixedOffset::east_opt(0).unwrap_or_else(|| unreachable!()),
                        )),
                        error_summary,
                    )
                    .await;

                    if let Some(mut worker) = state.scheduler.workers.get_mut(&current_worker_id) {
                        worker.status = if success {
                            WorkerStatus::Idle
                        } else {
                            WorkerStatus::Error(message)
                        };
                    }
                    state.scheduler.notify_task_available();
                }
                WSMessage::TaskPhaseUpdate { build_id, phase } => {
                    if let Some(mut worker) = state.scheduler.workers.get_mut(&current_worker_id)
                        && let WorkerStatus::Busy { build_id: id, .. } = &worker.status
                        && &build_id == id
                    {
                        worker.status = WorkerStatus::Busy {
                            build_id,
                            phase: Some(phase),
                        };
                    }
                }
                WSMessage::TargetBuildStatusBatch { events } => {
                    for update in events {
                        state.target_status_cache.insert_event(update).await;
                    }
                }
                _ => {}
            }
        }
        Message::Close(_) => {
            if let Some(id) = worker_id.take()
                && let Some(mut worker) = state.scheduler.workers.get_mut(&id)
            {
                worker.status = WorkerStatus::Lost;
            }
            return ControlFlow::Break(());
        }
        _ => {}
    }
    ControlFlow::Continue(())
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
