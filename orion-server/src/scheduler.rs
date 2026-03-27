use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

use api_model::buck2::{
    status::Status,
    types::{ProjectRelativePath, TaskPhase},
    ws::WSMessage,
};
use chrono::FixedOffset;
use dashmap::DashMap;
use rand::RngExt;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter as _, prelude::DateTimeUtc,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Notify, mpsc::UnboundedSender};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auto_retry::AutoRetryJudger,
    model::{dto::CoreWorkerStatus, internal::BuildTargetStateDTO, target_state::TargetState},
    repository::{build_events_repo::BuildEventsRepo, build_targets_repo::BuildTargetsRepo},
};

/// Task queue configuration
#[derive(Debug, Clone)]
pub struct TaskQueueConfig {
    /// Maximum queue length
    pub max_queue_size: usize,
    /// Maximum wait time for tasks in queue
    pub max_wait_time: Duration,
    /// Queue cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for TaskQueueConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            max_wait_time: Duration::from_secs(300), // 5 minutes
            cleanup_interval: Duration::from_secs(30), // Cleanup every 30 seconds
        }
    }
}

/// Simple FIFO task queue
#[derive(Debug)]
pub struct TaskQueue {
    /// Queue storage for v2 (should replace the original queue in the future)
    queue_v2: VecDeque<PendingBuildEventV2>,
    /// Queue configuration
    config: TaskQueueConfig,
}

impl TaskQueue {
    pub fn new(config: TaskQueueConfig) -> Self {
        Self {
            queue_v2: VecDeque::new(),
            config,
        }
    }

    pub fn enqueue_v2(&mut self, task: PendingBuildEventV2) -> Result<(), String> {
        // Check if queue is full
        if self.queue_v2.len() >= self.config.max_queue_size {
            return Err("Queue is full".to_string());
        }

        self.queue_v2.push_back(task);
        Ok(())
    }

    /// Clean up expired queued builds (v2)
    pub fn cleanup_expired_v2(&mut self) -> Vec<PendingBuildEventV2> {
        let now = Instant::now();
        let mut expired_tasks = Vec::new();

        self.queue_v2.retain(|task| {
            if now.duration_since(task.created_at) > self.config.max_wait_time {
                expired_tasks.push(task.clone());
                false
            } else {
                true
            }
        });

        expired_tasks
    }

    /// Get queue statistics
    pub fn get_stats(&self) -> TaskQueueStats {
        TaskQueueStats {
            total_queued: self.queue_v2.len(),
            oldest_task_age_seconds: self
                .queue_v2
                .front()
                .map(|task| Instant::now().duration_since(task.created_at).as_secs()),
        }
    }
}

/// Queue statistics
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TaskQueueStats {
    pub total_queued: usize,
    /// Age of oldest task in seconds
    pub oldest_task_age_seconds: Option<u64>,
}

/// Mandatory Information for building a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildEventPayload {
    pub build_event_id: Uuid,
    pub task_id: Uuid,
    pub cl_link: String,
    pub repo: String,
    pub retry_count: i32,
}

#[derive(Debug, Clone)]
// #[allow(dead_code)]
pub struct PendingBuildEventV2 {
    pub event_payload: BuildEventPayload,
    pub(crate) targets: Vec<BuildTargetStateDTO>,
    pub(crate) changes: Vec<Status<ProjectRelativePath>>,
    pub(crate) created_at: Instant,
}

/// Information for an active model
#[derive(Clone)]
pub struct BuildInfo {
    pub event_payload: BuildEventPayload,
    pub target_id: Uuid,
    #[allow(dead_code)]
    pub target_path: String,
    pub changes: Vec<Status<ProjectRelativePath>>,
    #[allow(dead_code)]
    pub started_at: DateTimeUtc,
    pub auto_retry_judger: AutoRetryJudger,
    pub worker_id: String,
}

impl BuildEventPayload {
    pub fn new(
        build_event_id: Uuid,
        task_id: Uuid,
        cl_link: String,
        repo: String,
        retry_count: i32,
    ) -> Self {
        Self {
            build_event_id,
            task_id,
            cl_link,
            repo,
            retry_count,
        }
    }
}

/// Status of a worker node
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub enum WorkerStatus {
    Idle,
    Busy {
        // Cont ains build ID when busy
        build_id: String,
        // Show task phase when needed
        phase: Option<TaskPhase>,
    },
    Error(String), // Contains fail message
    Lost,          // Heartbeat timeout
}

impl WorkerStatus {
    pub fn status_type(&self) -> CoreWorkerStatus {
        match self {
            WorkerStatus::Idle => CoreWorkerStatus::Idle,
            WorkerStatus::Busy { .. } => CoreWorkerStatus::Busy,
            WorkerStatus::Error(_) => CoreWorkerStatus::Error,
            WorkerStatus::Lost => CoreWorkerStatus::Lost,
        }
    }
}

/// Information about a connected worker
#[derive(Debug)]
pub struct WorkerInfo {
    pub sender: UnboundedSender<WSMessage>,
    pub status: WorkerStatus,
    pub last_heartbeat: DateTimeUtc,
    pub hostname: String,
    pub start_time: DateTimeUtc,
    pub orion_version: String,
}

/// Task scheduler - manages task queue and worker assignment
#[derive(Clone)]
pub struct TaskScheduler {
    /// Pending task queue
    pub pending_tasks: Arc<Mutex<TaskQueue>>,
    /// Event notifier for new tasks or available workers
    pub task_notifier: Arc<Notify>,
    /// Worker information
    pub workers: Arc<DashMap<String, WorkerInfo>>,
    /// Active build tasks
    pub active_builds: Arc<DashMap<String, BuildInfo>>,
    /// Database connection
    pub conn: DatabaseConnection,
}

impl TaskScheduler {
    /// Create new task scheduler instance
    pub fn new(
        conn: DatabaseConnection,
        workers: Arc<DashMap<String, WorkerInfo>>,
        active_builds: Arc<DashMap<String, BuildInfo>>,
        queue_config: Option<TaskQueueConfig>,
    ) -> Self {
        let config = queue_config.unwrap_or_default();
        Self {
            pending_tasks: Arc::new(Mutex::new(TaskQueue::new(config))),
            task_notifier: Arc::new(Notify::new()),
            workers,
            active_builds,
            conn,
        }
    }

    // NOTE: old `targets` table has been migrated away in favor of `callisto::build_targets`.

    pub async fn enqueue_task_v2(
        &self,
        task_id: Uuid,
        cl_link: &str,
        repo: String,
        changes: Vec<Status<ProjectRelativePath>>,
        retry_count: i32,
    ) -> Result<Uuid, String> {
        let build_event_id = Uuid::now_v7();

        self.enqueue_task_with_build_id_v2(
            build_event_id,
            task_id,
            cl_link,
            repo,
            changes,
            retry_count,
        )
        .await?;

        Ok(build_event_id)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn enqueue_task_with_build_id_v2(
        &self,
        build_event_id: Uuid,
        task_id: Uuid,
        cl_link: &str,
        repo: String,
        changes: Vec<Status<ProjectRelativePath>>,
        retry_count: i32,
    ) -> Result<(), String> {
        // Ensure the build event row exists in DB for queued tasks.
        BuildEventsRepo::insert_build(&self.conn, build_event_id, task_id, repo.clone())
            .await
            .map_err(|e| format!("Failed to insert build event: {e}"))?;

        // Ensure there is at least one default build target.
        let default_target_id = Uuid::now_v7();
        let _default_path =
            BuildTargetsRepo::insert_default_target(default_target_id, task_id, &self.conn)
                .await
                .map_err(|e| format!("Failed to insert default build target: {e}"))?;

        // Initialize a per-build target history record so UI can show "Pending" immediately.
        let now = chrono::Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        let _ = callisto::target_state_histories::ActiveModel {
            id: Set(Uuid::now_v7()),
            build_target_id: Set(default_target_id),
            build_event_id: Set(build_event_id),
            target_state: Set(TargetState::Pending.to_string()),
            created_at: Set(now),
        }
        .insert(&self.conn)
        .await;

        let targets =
            BuildTargetsRepo::find_initialized_build_targets(build_event_id, task_id, &self.conn)
                .await
                .map_err(|e| format!("Failed to find initialized build targets: {e}"))?;
        let event = BuildEventPayload::new(
            build_event_id,
            task_id,
            cl_link.to_string(),
            repo,
            retry_count,
        );

        let pending_build_event = PendingBuildEventV2 {
            event_payload: event,
            targets,
            changes,
            created_at: Instant::now(),
        };

        {
            let mut queue = self.pending_tasks.lock().await;
            queue.enqueue_v2(pending_build_event)?;
        }

        // Notify that there's a new task to process
        self.task_notifier.notify_one();
        Ok(())
    }

    /// Get queue statistics
    pub async fn get_queue_stats(&self) -> TaskQueueStats {
        let queue = self.pending_tasks.lock().await;
        queue.get_stats()
    }

    /// Clean up expired task-bound builds
    pub async fn cleanup_expired_tasks(&self) -> Vec<PendingBuildEventV2> {
        let mut queue = self.pending_tasks.lock().await;
        queue.cleanup_expired_v2()
    }

    /// Check if there are available workers
    pub fn has_idle_workers(&self) -> bool {
        self.workers
            .iter()
            .any(|entry| matches!(entry.value().status, WorkerStatus::Idle))
    }

    /// Get list of idle workers
    pub fn get_idle_workers(&self) -> Vec<String> {
        self.workers
            .iter()
            .filter(|entry| matches!(entry.value().status, WorkerStatus::Idle))
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Search available worker and claim the worker for current build
    pub fn search_and_claim_worker(&self, build_id: &str) -> Option<String> {
        let idle_workers: Vec<String> = self
            .workers
            .iter()
            .filter(|entry| matches!(entry.value().status, WorkerStatus::Idle))
            .map(|entry| entry.key().clone())
            .collect();
        let chosen_worker_idx = {
            let mut rng = rand::rng();
            rng.random_range(0..idle_workers.len())
        };
        let chosen_worker_id = idle_workers[chosen_worker_idx].clone();
        if let Some(mut worker) = self.workers.get_mut(&chosen_worker_id) {
            worker.status = WorkerStatus::Busy {
                build_id: build_id.to_string(),
                phase: None,
            };
            Some(chosen_worker_id)
        } else {
            None
        }
    }

    pub async fn release_worker(&self, worker_id: &str) {
        tracing::info!("Releasing worker {} back to idle", worker_id);
        if let Some(mut worker) = self.workers.get_mut(worker_id) {
            worker.status = WorkerStatus::Idle;
        }
    }

    /// Try to dispatch queued task-bound builds (concurrent safe)
    pub async fn process_pending_tasks(&self) {
        // Get available workers
        let idle_workers = self.get_idle_workers();
        if idle_workers.is_empty() {
            return;
        }

        // Process tasks in batches, up to the number of idle workers
        let max_tasks = idle_workers.len();
        let mut tasks_to_dispatch = Vec::with_capacity(max_tasks);

        // Batch dequeue tasks
        {
            let mut queue = self.pending_tasks.lock().await;
            for _ in 0..max_tasks {
                if let Some(task) = queue.queue_v2.pop_front() {
                    tasks_to_dispatch.push(task);
                } else {
                    break;
                }
            }
        }

        // Dispatch tasks concurrently
        if !tasks_to_dispatch.is_empty() {
            let dispatch_futures: Vec<_> = tasks_to_dispatch
                .into_iter()
                .map(|task| {
                    let scheduler = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = scheduler.dispatch_task_v2(task).await {
                            tracing::error!("Failed to dispatch queued task: {}", e);
                        }
                    })
                })
                .collect();

            // Wait for all dispatch tasks to complete
            for future in dispatch_futures {
                let _ = future.await;
            }
        }
    }

    /// Dispatch single queued v2 task
    async fn dispatch_task_v2(
        &self,
        pending_build_event: PendingBuildEventV2,
    ) -> Result<(), String> {
        let idle_workers = self.get_idle_workers();
        if idle_workers.is_empty() {
            return Err("No idle workers available".to_string());
        }

        // Randomly select an idle worker
        let chosen_index = {
            let mut rng = rand::rng();
            rng.random_range(0..idle_workers.len())
        };
        let chosen_id = idle_workers[chosen_index].clone();
        let start_at = chrono::Utc::now();
        let start_at_tz = start_at.with_timezone(&FixedOffset::east_opt(0).unwrap());

        // Choose a single default build target for this execution (current worker protocol builds one target).
        let target_id = pending_build_event
            .targets
            .first()
            .map(|t| t.id)
            .unwrap_or_else(Uuid::nil);
        let target_path = pending_build_event
            .targets
            .first()
            .map(|t| t.path.clone())
            .unwrap_or_else(|| "//".to_string());

        // Create build information (target_id is a build_target_id in the new schema)
        let build_info = BuildInfo {
            event_payload: pending_build_event.event_payload.clone(),
            changes: pending_build_event.changes.clone(),
            target_id,
            target_path: target_path.clone(),
            worker_id: chosen_id.clone(),
            auto_retry_judger: AutoRetryJudger::new(),
            started_at: start_at,
        };

        // Ensure a per-build target history row exists for this build/target.
        let _ = callisto::target_state_histories::ActiveModel {
            id: Set(Uuid::now_v7()),
            build_target_id: Set(target_id),
            build_event_id: Set(pending_build_event.event_payload.build_event_id),
            target_state: Set(TargetState::Building.to_string()),
            created_at: Set(start_at_tz),
        }
        .insert(&self.conn)
        .await;

        let _ = callisto::build_targets::Entity::update_many()
            .filter(callisto::build_targets::Column::Id.eq(target_id))
            .set(callisto::build_targets::ActiveModel {
                latest_state: Set(TargetState::Building.to_string()),
                ..Default::default()
            })
            .exec(&self.conn)
            .await;

        // Create WebSocket message
        let msg = WSMessage::TaskBuild {
            build_id: pending_build_event.event_payload.build_event_id.to_string(),
            repo: pending_build_event.event_payload.repo,
            cl_link: pending_build_event.event_payload.cl_link.to_string(),
            changes: pending_build_event.changes.clone(),
        };

        // Register the build *before* sending TaskBuild so early TaskBuildOutput lines
        // from the worker are not dropped (ws_handler only publishes logs when
        // active_builds contains the build_id).
        let build_key = pending_build_event.event_payload.build_event_id.to_string();
        if let Some(mut worker) = self.workers.get_mut(&chosen_id) {
            self.active_builds.insert(build_key.clone(), build_info);
            worker.status = WorkerStatus::Busy {
                build_id: build_key.clone(),
                phase: None,
            };
            if worker.sender.send(msg).is_err() {
                self.active_builds.remove(&build_key);
                worker.status = WorkerStatus::Idle;
                return Err(format!("Failed to send task to worker {chosen_id}"));
            }

            tracing::info!(
                "Queued task {}/{} dispatched to worker {}",
                pending_build_event.event_payload.task_id,
                pending_build_event.event_payload.build_event_id,
                chosen_id
            );
            Ok(())
        } else {
            Err(format!("Worker {chosen_id} not found"))
        }
    }

    /// Notify about new task or available worker
    pub fn notify_task_available(&self) {
        self.task_notifier.notify_one();
    }

    /// Start queue management background task (event-driven + periodic cleanup)
    pub async fn start_queue_manager(self) {
        let cleanup_interval = {
            let queue = self.pending_tasks.lock().await;
            queue.config.cleanup_interval
        };

        // Task dispatcher: wait for notifications or process periodically
        let dispatch_scheduler = self.clone();
        let dispatch_task = tokio::spawn(async move {
            loop {
                // Wait for notification or timeout
                tokio::select! {
                    // Wait for new task or worker available notification
                    _ = dispatch_scheduler.task_notifier.notified() => {
                        dispatch_scheduler.process_pending_tasks().await;
                    }
                    // Periodic check (prevent missing notifications)
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {
                        dispatch_scheduler.process_pending_tasks().await;
                    }
                }
            }
        });

        // Cleaner: periodically clean up expired tasks
        let cleanup_scheduler = self.clone();
        let cleanup_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);

            loop {
                interval.tick().await;

                // Clean up expired tasks
                let expired_tasks = cleanup_scheduler.cleanup_expired_tasks().await;
                if !expired_tasks.is_empty() {
                    tracing::warn!(
                        "Cleaned up {} expired tasks from queue",
                        expired_tasks.len()
                    );

                    // Log expired task information
                    for task in expired_tasks {
                        tracing::debug!(
                            "Expired build: {}/{} ({})",
                            task.event_payload.task_id,
                            task.event_payload.build_event_id,
                            task.event_payload.repo
                        );
                    }
                }
            }
        });

        // Wait for tasks to complete (actually runs forever)
        tokio::select! {
            _ = dispatch_task => {
                tracing::error!("Task dispatcher unexpectedly stopped");
            }
            _ = cleanup_task => {
                tracing::error!("Task cleanup unexpectedly stopped");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_capacity_v2() {
        let config = TaskQueueConfig {
            max_queue_size: 2,
            max_wait_time: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(30),
        };
        let mut queue = TaskQueue::new(config);

        let build_event = BuildEventPayload::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            "test_cl_link".to_string(),
            "test/repo".to_string(),
            0,
        );
        let task = PendingBuildEventV2 {
            event_payload: build_event.clone(),
            targets: vec![],
            changes: vec![],
            created_at: Instant::now(),
        };

        // Fill queue to capacity
        assert!(queue.enqueue_v2(task.clone()).is_ok());
        assert!(queue.enqueue_v2(task.clone()).is_ok());

        // Should fail when full
        assert!(queue.enqueue_v2(task).is_err());
    }
}
