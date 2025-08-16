use crate::model::builds;
use dashmap::DashMap;
use orion::ws::WSMessage;
use rand::Rng;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, prelude::DateTimeUtc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::SeekFrom;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::{Mutex, Notify, mpsc::UnboundedSender};
use utoipa::ToSchema;
use uuid::Uuid;

/// Request payload for creating a new build task
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[allow(dead_code)]
pub struct BuildRequest {
    pub repo: String,
    pub buck_hash: String,
    pub buckconfig_hash: String,
    pub args: Option<Vec<String>>,
    pub mr: Option<String>,
}

/// Pending task waiting for dispatch
#[derive(Debug, Clone)]
pub struct PendingTask {
    pub task_id: Uuid,
    pub request: BuildRequest,
    pub target: String,
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

    /// Add task to the end of queue
    pub fn enqueue(&mut self, task: PendingTask) -> Result<(), String> {
        // Check if queue is full
        if self.queue.len() >= self.config.max_queue_size {
            return Err("Queue is full".to_string());
        }

        self.queue.push_back(task);
        Ok(())
    }

    /// Remove task from the front of queue
    pub fn dequeue(&mut self) -> Option<PendingTask> {
        self.queue.pop_front()
    }

    /// Clean up expired tasks
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
pub struct BuildInfo {
    pub repo: String,
    pub target: String,
    pub args: Option<Vec<String>>,
    pub start_at: DateTimeUtc,
    pub mr: Option<String>,
    pub _worker_id: String,
    pub log_file: Arc<Mutex<std::fs::File>>,
}

/// Status of a worker node
#[derive(Debug, Clone)]
pub enum WorkerStatus {
    Idle,
    Busy(String), // Contains task ID when busy
}

/// Information about a connected worker
#[derive(Debug)]
pub struct WorkerInfo {
    pub sender: UnboundedSender<WSMessage>,
    pub status: WorkerStatus,
    pub last_heartbeat: DateTimeUtc,
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
    /// Task id / log file name
    pub task_id: String,
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

    /// Read a segment of a log file by task id at given offset, limited to max_len bytes.
    pub async fn read_log_segment(
        &self,
        task_id: &str,
        offset: u64,
        max_len: usize,
    ) -> Result<LogSegment, LogReadError> {
        read_log_segment_raw(task_id, offset, max_len).await
    }

    /// Add task to queue
    pub async fn enqueue_task(
        &self,
        request: BuildRequest,
        target: String,
    ) -> Result<Uuid, String> {
        let task_id = Uuid::now_v7();

        let pending_task = PendingTask {
            task_id,
            request,
            target,
            created_at: Instant::now(),
        };

        {
            let mut queue = self.pending_tasks.lock().await;
            queue.enqueue(pending_task)?;
        }

        // Notify that there's a new task to process
        self.task_notifier.notify_one();
        Ok(task_id)
    }

    /// Get queue statistics
    pub async fn get_queue_stats(&self) -> TaskQueueStats {
        let queue = self.pending_tasks.lock().await;
        queue.get_stats()
    }

    /// Clean up expired tasks
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

    /// Try to dispatch queued tasks (concurrent safe)
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

        // Create log file
        let log_file = match create_log_file(&pending_task.task_id.to_string()) {
            Ok(file) => Arc::new(Mutex::new(file)),
            Err(e) => {
                tracing::error!(
                    "Failed to create log file for task {}: {}",
                    pending_task.task_id,
                    e
                );
                return Err(format!("Failed to create log file: {e}"));
            }
        };

        // Create build information
        let build_info = BuildInfo {
            repo: pending_task.request.repo.clone(),
            target: pending_task.target.clone(),
            args: pending_task.request.args.clone(),
            start_at: chrono::Utc::now(),
            mr: pending_task.request.mr.clone(),
            _worker_id: chosen_id.clone(),
            log_file,
        };

        // Save to database
        let model = builds::ActiveModel {
            build_id: Set(pending_task.task_id),
            output_file: Set(format!("{}/{}", get_build_log_dir(), pending_task.task_id)),
            exit_code: Set(None),
            start_at: Set(build_info.start_at),
            end_at: Set(None),
            repo_name: Set(build_info.repo.clone()),
            target: Set(build_info.target.clone()),
            arguments: Set(build_info.args.clone().unwrap_or_default().join(" ")),
            mr: Set(build_info.mr.clone().unwrap_or_default()),
        };

        if let Err(e) = model.insert(&self.conn).await {
            tracing::error!("Failed to insert queued task into DB: {}", e);
            return Err(format!("Failed to create task in database: {e}"));
        }

        // Create WebSocket message
        let msg = WSMessage::Task {
            id: pending_task.task_id.to_string(),
            repo: pending_task.request.repo,
            target: pending_task.target,
            args: pending_task.request.args,
            mr: pending_task.request.mr.unwrap_or_default(),
        };

        // Send task to worker
        if let Some(mut worker) = self.workers.get_mut(&chosen_id) {
            if worker.sender.send(msg).is_ok() {
                worker.status = WorkerStatus::Busy(pending_task.task_id.to_string());
                self.active_builds
                    .insert(pending_task.task_id.to_string(), build_info);
                tracing::info!(
                    "Queued task {} dispatched to worker {}",
                    pending_task.task_id,
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
                        tracing::debug!("Expired task: {} ({})", task.task_id, task.request.repo);
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

/// Read a segment of a task log file.
/// Returns metadata and data slice (UTF-8 lossy converted).
pub async fn read_log_segment_raw(
    task_id: &str,
    offset: u64,
    max_len: usize,
) -> Result<LogSegment, LogReadError> {
    let log_path = format!("{}/{}", get_build_log_dir(), task_id);
    let path = std::path::Path::new(&log_path);
    if !path.exists() {
        return Err(LogReadError::NotFound);
    }

    let meta = tokio::fs::metadata(path).await.map_err(LogReadError::Io)?;
    let size = meta.len();
    if offset > size {
        return Err(LogReadError::OffsetOutOfRange { size });
    }

    // Fast path: only metadata
    if max_len == 0 || offset == size {
        return Ok(LogSegment {
            task_id: task_id.to_string(),
            offset,
            len: 0,
            data: String::new(),
            next_offset: offset,
            file_size: size,
            eof: offset >= size,
        });
    }

    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(LogReadError::Io)?;
    file.seek(SeekFrom::Start(offset))
        .await
        .map_err(LogReadError::Io)?;

    let remaining = (size - offset) as usize;
    let to_read = remaining.min(max_len);
    let mut buf = vec![0u8; to_read];
    let read_bytes = file.read(&mut buf).await.map_err(LogReadError::Io)?;
    buf.truncate(read_bytes);
    let data = String::from_utf8_lossy(&buf).to_string();
    let next_offset = offset + read_bytes as u64;
    let eof = next_offset >= size;

    Ok(LogSegment {
        task_id: task_id.to_string(),
        offset,
        len: read_bytes,
        data,
        next_offset,
        file_size: size,
        eof,
    })
}

/// Unified accessor for the build log directory (BUILD_LOG_DIR).
///
/// Behavior differs between test and non-test builds:
///
/// Non-test (`cfg(not(test))`):
///   * Uses `once_cell::sync::Lazy` to read the env var exactly once at first access.
///   * Panics early if the variable is missing (surfacing deployment misconfiguration).
///   * Cannot be changed at runtime (subsequent env var edits are ignored).
///
/// Test (`cfg(test)`):
///   * Allows setting `BUILD_LOG_DIR` before the first call in each test thread.
///   * Uses `thread_local!` + `Cell<Option<&'static str>>`; on first access leaks the string via `Box::leak` only once per thread (bounded leak acceptable in tests).
///   * Motivation:
///       - Avoid a global `Lazy` capturing a temporary directory too early for all tests.
///       - Keep memory growth bounded (one leaked string per thread at most).
///   * Changing the environment variable in the same thread after first access has no effect.
///
/// Usage in tests:
/// ```ignore
/// let tmp = tempfile::tempdir().unwrap();
/// std::env::set_var("BUILD_LOG_DIR", tmp.path());
/// let dir = get_build_log_dir();
/// ```
///
/// # Panics
/// Panics if `BUILD_LOG_DIR` is not set at first access.
///
/// # Thread Safety
/// Returns an immutable `&'static str`. Non-test mode uses a `Lazy` (thread-safe once init);
/// test mode uses per-thread initialization to avoid cross-thread contention / early capture.
///
/// # Possible Future Improvement
/// If hot-swapping the directory is ever required, this could return `Arc<PathBuf>` and expose
/// an atomic update mechanism. Current requirements favor simplicity and immutability.
pub fn get_build_log_dir() -> &'static str {
    // Body only distinguishes cfg paths; see doc comment above for detailed rationale.
    #[cfg(not(test))]
    {
        use once_cell::sync::Lazy;
        static BUILD_LOG_DIR: Lazy<String> =
            Lazy::new(|| std::env::var("BUILD_LOG_DIR").expect("BUILD_LOG_DIR must be set"));
        &BUILD_LOG_DIR
    }
    #[cfg(test)]
    {
        // Test mode: allow setting BUILD_LOG_DIR before first use; only leak once per thread.
        use std::cell::Cell;
        thread_local! {
            static BUILD_LOG_DIR_TLS: Cell<Option<&'static str>> = const { Cell::new(None) };
        }
        BUILD_LOG_DIR_TLS.with(|cell| {
            if cell.get().is_none() {
                let val = std::env::var("BUILD_LOG_DIR").expect("BUILD_LOG_DIR must be set");
                let leaked: &'static str = Box::leak(val.into_boxed_str());
                cell.set(Some(leaked));
            }
            cell.get().unwrap()
        })
    }
}

/// Create log file
pub fn create_log_file(task_id: &str) -> Result<std::fs::File, std::io::Error> {
    let log_path = format!("{}/{}", get_build_log_dir(), task_id);
    let path = std::path::Path::new(&log_path);

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create or open the log file in append mode
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Test task queue basic functionality
    #[test]
    fn test_task_queue_fifo() {
        let config = TaskQueueConfig::default();
        let mut queue = TaskQueue::new(config);

        // Create test tasks
        let task1 = PendingTask {
            task_id: Uuid::now_v7(),
            request: BuildRequest {
                repo: "test1".to_string(),
                buck_hash: "hash1".to_string(),
                buckconfig_hash: "config1".to_string(),
                args: None,
                mr: None,
            },
            target: "target1".to_string(),
            created_at: Instant::now(),
        };

        let task2 = PendingTask {
            task_id: Uuid::now_v7(),
            request: BuildRequest {
                repo: "test2".to_string(),
                buck_hash: "hash2".to_string(),
                buckconfig_hash: "config2".to_string(),
                args: None,
                mr: None,
            },
            target: "target2".to_string(),
            created_at: Instant::now(),
        };

        // Test FIFO behavior
        assert!(queue.enqueue(task1.clone()).is_ok());
        assert!(queue.enqueue(task2.clone()).is_ok());

        let dequeued1 = queue.dequeue().unwrap();
        assert_eq!(dequeued1.task_id, task1.task_id);
        assert_eq!(dequeued1.request.repo, "test1");

        let dequeued2 = queue.dequeue().unwrap();
        assert_eq!(dequeued2.task_id, task2.task_id);
        assert_eq!(dequeued2.request.repo, "test2");
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
            request: BuildRequest {
                repo: "test".to_string(),
                buck_hash: "hash".to_string(),
                buckconfig_hash: "config".to_string(),
                args: None,
                mr: None,
            },
            target: "target".to_string(),
            created_at: Instant::now(),
        };

        // Fill queue to capacity
        assert!(queue.enqueue(task.clone()).is_ok());
        assert!(queue.enqueue(task.clone()).is_ok());

        // Should fail when full
        assert!(queue.enqueue(task).is_err());
    }

    #[tokio::test]
    async fn test_read_log_segment_basic() {
        // Prepare temp dir
        let tmp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("BUILD_LOG_DIR", tmp.path().to_str().unwrap());
        }
        let task_id = "segment-test";
        let mut file = create_log_file(task_id).unwrap();
        write!(file, "Hello World! This is a test log.").unwrap();

        // Read first 5 bytes
        let seg = read_log_segment_raw(task_id, 0, 5).await.unwrap();
        assert_eq!(seg.offset, 0);
        assert_eq!(seg.len, 5);
        assert_eq!(seg.data, "Hello");
        assert!(!seg.eof);

        // Read next bytes
        let seg2 = read_log_segment_raw(task_id, seg.next_offset, 100)
            .await
            .unwrap();
        assert!(seg2.data.starts_with(" World"));
    }

    #[tokio::test]
    async fn test_read_log_segment_offset_out_of_range() {
        let tmp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("BUILD_LOG_DIR", tmp.path().to_str().unwrap());
        }
        let task_id = "segment-oob";
        let _ = create_log_file(task_id).unwrap();
        let res = read_log_segment_raw(task_id, 10, 10).await;
        assert!(matches!(res, Err(LogReadError::OffsetOutOfRange { .. })));
    }
}
