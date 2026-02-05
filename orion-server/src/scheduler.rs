use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

use api_model::buck2::{
    types::{ProjectRelativePath, Status},
    ws::WSMessage,
};
use chrono::FixedOffset;
use dashmap::DashMap;
use orion::ws::{TaskPhase, WSMessage};
use rand::Rng;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, prelude::DateTimeUtc};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Notify, mpsc::UnboundedSender};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::CoreWorkerStatus,
    auto_retry::AutoRetryJudger,
    log::log_service::LogService,
    model::{builds, targets, targets::TargetState},
};

/// Request payload for creating a new build task
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BuildRequest {
    pub changes: Vec<Status<ProjectRelativePath>>,
    /// Buck2 target path (e.g. //app:server). Optional for backward compatibility.
    #[serde(default, alias = "target_path")]
    pub target: Option<String>,
}

impl BuildRequest {
    /// Return requested target path; fallback to "//..." for backward compatibility.
    pub fn target_path(&self) -> String {
        self.target
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "//...".to_string())
    }
}

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
    /// Queue storage (FIFO)
    queue: VecDeque<PendingBuildEvent>,
    /// Queue configuration
    config: TaskQueueConfig,
}

impl TaskQueue {
    pub fn new(config: TaskQueueConfig) -> Self {
        Self {
            queue: VecDeque::new(),
            config,
        }
    }

    /// Add task-bound build to the end of queue
    pub fn enqueue(&mut self, task: PendingBuildEvent) -> Result<(), String> {
        // Check if queue is full
        if self.queue.len() >= self.config.max_queue_size {
            return Err("Queue is full".to_string());
        }

        self.queue.push_back(task);
        Ok(())
    }

    /// Remove task-bound build from the front of queue
    pub fn dequeue(&mut self) -> Option<PendingBuildEvent> {
        self.queue.pop_front()
    }

    /// Clean up expired task-bound build
    pub fn cleanup_expired(&mut self) -> Vec<PendingBuildEvent> {
        let now = Instant::now();
        let mut expired_tasks = Vec::new();

        self.queue.retain(|task| {
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
            total_queued: self.queue.len(),
            oldest_task_age_seconds: self
                .queue
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BuildEventPayload {
    pub build_event_id: Uuid,
    pub task_id: Uuid,
    pub cl_link: String,
    pub repo: String,
    pub retry_count: i32,
}

/// Pending task waiting for dispatch
#[derive(Debug, Clone)]
pub struct PendingBuildEvent {
    pub event_payload: BuildEventPayload,
    pub target_id: Option<Uuid>,
    pub target_path: Option<String>,
    pub changes: Vec<Status<ProjectRelativePath>>,
    pub created_at: Instant,
}

/// Information for an active model
#[derive(Clone)]
pub struct BuildInfo {
    pub event_payload: BuildEventPayload,
    pub target_id: Uuid,
    pub target_path: String,
    pub changes: Vec<Status<ProjectRelativePath>>,
    pub started_at: DateTimeUtc,
    pub auto_retry_judger: AutoRetryJudger,
    pub _worker_id: String,
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
        // Contains task ID when busy
        task_id: String,
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

/// Errors when reading a log segment
#[allow(dead_code)]
#[derive(Debug)]
pub enum LogReadError {
    NotFound,
    OffsetOutOfRange { size: u64 },
    Io(std::io::Error),
}

impl From<std::io::Error> for LogReadError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
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

    pub async fn ensure_target(
        &self,
        task_id: Uuid,
        target_path: &str,
    ) -> Result<targets::Model, sea_orm::DbErr> {
        // Find-or-create target for (task_id, target_path)
        targets::Entity::find_or_create(&self.conn, task_id, target_path.to_string()).await
    }

    /// Bound corresponding task build ID to the given task and enqueue
    /// Used when idle worker is not available
    pub async fn enqueue_task(
        &self,
        task_id: Uuid,
        cl_link: &str,
        repo: String,
        changes: Vec<Status<ProjectRelativePath>>,
        retry_count: i32,
    ) -> Result<Uuid, String> {
        let build_event_id = Uuid::now_v7();

        self.enqueue_task_with_build_id(
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

    /// Enqueue task build with given BuildEvent ID
    #[allow(clippy::too_many_arguments)]
    pub async fn enqueue_task_with_build_id(
        &self,
        build_event_id: Uuid,
        task_id: Uuid,
        cl_link: &str,
        repo: String,
        changes: Vec<Status<ProjectRelativePath>>,
        retry_count: i32,
    ) -> Result<(), String> {
        // TODO: replace with the new target model
        let target_model = self
            .ensure_target(task_id, &target_path)
            .await
            .map_err(|e| e.to_string())?;
        let event = BuildEventPayload::new(
            build_event_id,
            task_id,
            cl_link.to_string(),
            repo.clone(),
            retry_count,
        );

        let pending_build_event = PendingBuildEvent {
            event_payload: event,
            target_id: None,
            target_path: None,
            changes: changes,
            created_at: Instant::now(),
        };

        {
            let mut queue = self.pending_tasks.lock().await;
            queue.enqueue(pending_build_event)?;
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
    pub async fn cleanup_expired_tasks(&self) -> Vec<PendingBuildEvent> {
        let mut queue = self.pending_tasks.lock().await;
        queue.cleanup_expired()
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
                if let Some(task) = queue.dequeue() {
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
                        if let Err(e) = scheduler.dispatch_task(task).await {
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

    /// Dispatch single task
    async fn dispatch_task(&self, pending_build_event: PendingBuildEvent) -> Result<(), String> {
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

        // Create build information
        let build_info = BuildInfo {
            event_payload: pending_build_event.event_payload.clone(),
            changes: pending_build_event.changes.clone(),
            target_id: pending_build_event
                .target_id
                .map_or_else(|| Uuid::nil(), |id| id),
            target_path: pending_build_event
                .target_path
                .clone()
                .unwrap_or_else(|| "".to_string()),
            _worker_id: chosen_id.clone(),
            auto_retry_judger: AutoRetryJudger::new(),
            started_at: start_at,
        };

        // Insert build record (fail fast if insertion fails)
        if let Err(e) = (builds::ActiveModel {
            id: Set(pending_build_event.build_event_id),
            task_id: Set(pending_build_event.task_id),
            // target_id: Set(pending_build_event.target_id),
            target_id: Set(None),
            exit_code: Set(None),
            start_at: Set(start_at_tz),
            end_at: Set(None),
            repo: Set(build_info.repo.clone()),
            args: Set(None),
            output_file: Set(format!(
                "{}/{}/{}.log",
                pending_build_event.task_id,
                LogService::last_segment(&pending_build_event.repo),
                pending_build_event.build_event_id
            )),
            created_at: Set(start_at_tz),
            retry_count: Set(0),
        })
        .insert(&self.conn)
        .await
        {
            tracing::error!(
                "Failed to insert build {} for task {}: {}",
                pending_build_event.build_event_id,
                pending_build_event.task_id,
                e
            );
            return Err(format!(
                "Failed to insert build {}",
                pending_build_event.build_event_id
            ));
        }

        println!("insert build");

        // Create WebSocket message
        let msg = WSMessage::Task {
            id: pending_build_event.build_event_id.to_string(),
            repo: pending_build_event.repo,
            cl_link: pending_build_event.cl_link.to_string(),
            changes: pending_build_event.changes.clone(),
        };

        // Send task to worker
        if let Some(mut worker) = self.workers.get_mut(&chosen_id) {
            if worker.sender.send(msg).is_ok() {
                // Only mark Building after send succeeds
                if let Err(e) = targets::update_state(
                    &self.conn,
                    //TODO: update target_id here
                    // pending_task.target_id,
                    0,
                    TargetState::Building,
                    Some(start_at_tz),
                    None,
                    None,
                )
                .await
                {
                    tracing::warn!("update target state failed: {e}");
                }

                worker.status = WorkerStatus::Busy {
                    task_id: pending_build_event.build_event_id.to_string(),
                    phase: None,
                };
                self.active_builds
                    .insert(pending_build_event.build_event_id.to_string(), build_info);
                tracing::info!(
                    "Queued task {}/{} dispatched to worker {}",
                    pending_build_event.task_id,
                    pending_build_event.build_event_id,
                    chosen_id
                );
                Ok(())
            } else {
                // Send failed: best-effort mark target back to Pending
                let _ = targets::update_state(
                    &self.conn,
                    // TODO: update target_id here
                    pending_build_event.target_id,
                    TargetState::Pending,
                    Some(start_at_tz),
                    None,
                    None,
                )
                .await
                .map_err(|e| tracing::warn!("update target rollback failed: {e}"));
                Err(format!("Failed to send task to worker {chosen_id}"))
            }
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
                            task.task_id,
                            task.build_event_id,
                            task.repo
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

    /// Test task queue basic functionality
    #[test]
    fn test_task_queue_fifo() {
        let config = TaskQueueConfig::default();
        let mut queue = TaskQueue::new(config);

        let build_event1 = BuildEventPayload::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            "test_cl_link".to_string(),
            "test/repo".to_string(),
            0,
        );

        // Create test tasks
        let task1 = PendingBuildEvent {
            event_payload: build_event1.clone(),
            target_id: Some(Uuid::now_v7()),
            target_path: Some("//app:server".to_string()),
            changes: vec![],
            created_at: Instant::now(),
        };

        let build_event2 = BuildEventPayload::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            "test_cl_link_2".to_string(),
            "test2/repo".to_string(),
            0,
        );
        let task2 = PendingBuildEvent {
            event_payload: build_event2.clone(),
            target_id: Some(Uuid::now_v7()),
            target_path: Some("//app:server2".to_string()),
            changes: vec![],
            created_at: Instant::now(),
        };

        // Test FIFO behavior
        assert!(queue.enqueue(task1.clone()).is_ok());
        assert!(queue.enqueue(task2.clone()).is_ok());

        let dequeued1 = queue.dequeue().unwrap();
        assert_eq!(dequeued1.build_event_id, task1.build_event_id);
        assert_eq!(dequeued1.repo, "/test/repo");

        let dequeued2 = queue.dequeue().unwrap();
        assert_eq!(dequeued2.build_event_id, task2.build_event_id);
        assert_eq!(dequeued2.repo, "/test2/repo");
    }

    /// Test queue capacity limit
    #[test]
    fn test_queue_capacity() {
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
        let task = PendingBuildEvent {
            event_payload: build_event.clone(),
            target_id: Some(Uuid::now_v7()),
            target_path: Some("//app:server".to_string()),
            changes: vec![],
            created_at: Instant::now(),
        };

        // Fill queue to capacity
        assert!(queue.enqueue(task.clone()).is_ok());
        assert!(queue.enqueue(task.clone()).is_ok());

        // Should fail when full
        assert!(queue.enqueue(task).is_err());
    }
}
