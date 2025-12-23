use crate::log::log_service::LogService;
use crate::model::builds;
use chrono::FixedOffset;
use dashmap::DashMap;
use orion::repo::sapling::status::{ProjectRelativePath, Status};
use orion::ws::{TaskPhase, WSMessage};
use rand::Rng;
use sea_orm::ActiveModelTrait;
use sea_orm::{ActiveValue::Set, DatabaseConnection, prelude::DateTimeUtc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Notify, mpsc::UnboundedSender};
use utoipa::ToSchema;
use uuid::Uuid;

/// Request payload for creating a new build task
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BuildRequest {
    pub changes: Vec<Status<ProjectRelativePath>>,
}

/// Pending task waiting for dispatch
#[derive(Debug, Clone)]
pub struct PendingTask {
    pub task_id: Uuid,
    pub cl_link: String,
    pub build_id: Uuid,
    pub repo: String,
    pub cl: i64,
    pub request: BuildRequest,
    pub created_at: Instant,
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
    queue: VecDeque<PendingTask>,
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
    pub fn enqueue(&mut self, task: PendingTask) -> Result<(), String> {
        // Check if queue is full
        if self.queue.len() >= self.config.max_queue_size {
            return Err("Queue is full".to_string());
        }

        self.queue.push_back(task);
        Ok(())
    }

    /// Remove task-bound build from the front of queue
    pub fn dequeue(&mut self) -> Option<PendingTask> {
        self.queue.pop_front()
    }

    /// Clean up expired task-bound build
    pub fn cleanup_expired(&mut self) -> Vec<PendingTask> {
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

/// Information about an active build task
#[derive(Clone)]
#[allow(dead_code)]
pub struct BuildInfo {
    pub task_id: String,
    pub build_id: String,
    pub repo: String,
    pub start_at: DateTimeUtc,
    pub changes: Vec<Status<ProjectRelativePath>>,
    pub cl: String,
    pub _worker_id: String,
}

/// Status of a worker node
#[derive(Debug, Clone, Serialize, ToSchema)]
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

/// Log segment read result
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LogSegment {
    /// build id / log file name
    pub build_id: String,
    /// Requested starting offset
    pub offset: u64,
    /// Bytes actually read
    pub len: usize,
    /// UTF-8 (lossy) decoded data slice
    pub data: String,
    /// Next offset (offset + len)
    pub next_offset: u64,
    /// Total file size in bytes
    pub file_size: u64,
    /// Whether we reached end of file
    pub eof: bool,
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

    /// Add task-bound build to queue
    pub async fn enqueue_task(
        &self,
        task_id: Uuid,
        cl_link: &str,
        request: BuildRequest,
        repo: String,
        cl: i64,
    ) -> Result<Uuid, String> {
        let build_id = Uuid::now_v7();

        let pending_task = PendingTask {
            task_id,
            cl_link: cl_link.to_string(),
            build_id,
            request,
            created_at: Instant::now(),
            repo,
            cl,
        };

        {
            let mut queue = self.pending_tasks.lock().await;
            queue.enqueue(pending_task)?;
        }

        // Notify that there's a new task to process
        self.task_notifier.notify_one();
        Ok(build_id)
    }

    /// Get queue statistics
    pub async fn get_queue_stats(&self) -> TaskQueueStats {
        let queue = self.pending_tasks.lock().await;
        queue.get_stats()
    }

    /// Clean up expired task-bound builds
    pub async fn cleanup_expired_tasks(&self) -> Vec<PendingTask> {
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
    async fn dispatch_task(&self, pending_task: PendingTask) -> Result<(), String> {
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

        // Create build information
        let build_info = BuildInfo {
            task_id: pending_task.task_id.to_string(),
            build_id: pending_task.build_id.to_string(),
            repo: pending_task.repo.clone(),
            start_at: chrono::Utc::now(),
            changes: pending_task.request.changes.clone(),
            cl: pending_task.cl.to_string(),
            _worker_id: chosen_id.clone(),
        };

        // Insert build record
        let _ = builds::ActiveModel {
            id: Set(pending_task.build_id),
            task_id: Set(pending_task.task_id),
            exit_code: Set(None),
            start_at: Set(build_info
                .start_at
                .with_timezone(&FixedOffset::east_opt(0).unwrap())),
            end_at: Set(None),
            repo: Set(build_info.repo.clone()),
            target: Set("//...".to_string()),
            args: Set(None),
            output_file: Set(format!(
                "{}/{}/{}.log",
                pending_task.task_id,
                LogService::last_segment(&pending_task.repo),
                pending_task.build_id
            )),
            created_at: Set(build_info
                .start_at
                .with_timezone(&FixedOffset::east_opt(0).unwrap())),
        }
        .insert(&self.conn)
        .await;

        println!("insert build");

        // Create WebSocket message
        let msg = WSMessage::Task {
            id: pending_task.build_id.to_string(),
            repo: pending_task.repo,
            cl_link: pending_task.cl_link.to_string(),
            changes: pending_task.request.changes.clone(),
        };

        // Send task to worker
        if let Some(mut worker) = self.workers.get_mut(&chosen_id) {
            if worker.sender.send(msg).is_ok() {
                worker.status = WorkerStatus::Busy {
                    task_id: pending_task.build_id.to_string(),
                    phase: None,
                };
                self.active_builds
                    .insert(pending_task.build_id.to_string(), build_info);
                tracing::info!(
                    "Queued task {}/{} dispatched to worker {}",
                    pending_task.task_id,
                    pending_task.build_id,
                    chosen_id
                );
                Ok(())
            } else {
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
                            task.build_id,
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

        // Create test tasks
        let task1 = PendingTask {
            task_id: Uuid::now_v7(),
            build_id: Uuid::now_v7(),
            request: BuildRequest { changes: vec![] },
            created_at: Instant::now(),
            repo: "/test/repo".to_string(),
            cl: 123456,
            cl_link: "test".to_string(),
        };

        let task2 = PendingTask {
            task_id: Uuid::now_v7(),
            build_id: Uuid::now_v7(),
            request: BuildRequest { changes: vec![] },
            created_at: Instant::now(),
            repo: "/test2/repo".to_string(),
            cl: 123457,
            cl_link: "test".to_string(),
        };

        // Test FIFO behavior
        assert!(queue.enqueue(task1.clone()).is_ok());
        assert!(queue.enqueue(task2.clone()).is_ok());

        let dequeued1 = queue.dequeue().unwrap();
        assert_eq!(dequeued1.build_id, task1.build_id);
        assert_eq!(dequeued1.repo, "/test/repo");

        let dequeued2 = queue.dequeue().unwrap();
        assert_eq!(dequeued2.build_id, task2.build_id);
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

        let task = PendingTask {
            task_id: Uuid::now_v7(),
            build_id: Uuid::now_v7(),
            request: BuildRequest { changes: vec![] },
            created_at: Instant::now(),
            repo: "/test/repo".to_string(),
            cl: 123456,
            cl_link: "test".to_string(),
        };

        // Fill queue to capacity
        assert!(queue.enqueue(task.clone()).is_ok());
        assert!(queue.enqueue(task.clone()).is_ok());

        // Should fail when full
        assert!(queue.enqueue(task).is_err());
    }
}
