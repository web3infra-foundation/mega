use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time;
use uuid::Uuid;
use serde::Serialize;

use crate::api::{BuildRequest, WorkerStatus, AppState};
use orion::ws::WSMessage;

/// State of queued tasks
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    /// Waiting for scheduling
    Pending,
    /// Assigned to a worker
    Assigned(String), // worker_id
    /// Currently running
    Running(String),  // worker_id
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Timed out
    Timeout,
    /// Cancelled
    Cancelled,
}

/// Task item in the queue
#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub task_id: Uuid,
    pub build_request: BuildRequest,
    pub target: String, // buck2 targets
    pub state: TaskState,
    pub created_at: Instant,
    pub timeout_duration: Duration,
    pub retry_count: u32,
    pub max_retries: u32,
    pub assigned_worker: Option<String>,
    pub priority: TaskPriority,
}

/// Task priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

impl QueuedTask {
    /// Create a new QueuedTask from BuildRequest
    pub fn new(
        build_request: BuildRequest,
        target: String,
        config: &SchedulerConfig,
        priority: Option<TaskPriority>,
    ) -> Self {
        Self {
            task_id: Uuid::now_v7(),
            build_request,
            target,
            state: TaskState::Pending,
            created_at: Instant::now(),
            timeout_duration: config.default_task_timeout,
            retry_count: 0,
            max_retries: config.default_max_retries,
            assigned_worker: None,
            priority: priority.unwrap_or_default(),
        }
    }

    /// Check if task has timed out
    pub fn is_timed_out(&self) -> bool {
        Instant::now().duration_since(self.created_at) > self.timeout_duration
    }

    /// Check if task can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Reset task for retry
    pub fn reset_for_retry(&mut self) {
        self.retry_count += 1;
        self.state = TaskState::Pending;
        self.assigned_worker = None;
        self.created_at = Instant::now();
    }
}

/// Scheduler configuration
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum queue length
    pub max_queue_length: usize,
    /// Default task timeout duration
    pub default_task_timeout: Duration,
    /// Default maximum retry count
    pub default_max_retries: u32,
    /// Scheduler check interval
    pub scheduler_interval: Duration,
    /// Task cleanup interval
    pub cleanup_interval: Duration,
    /// Retention time for completed tasks
    pub completed_task_retention: Duration,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_queue_length: 1000,
            default_task_timeout: Duration::from_secs(3600), // 1 hour
            default_max_retries: 3,
            scheduler_interval: Duration::from_millis(100),
            cleanup_interval: Duration::from_secs(60),
            completed_task_retention: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Scheduler statistics
#[derive(Debug, Clone, Serialize)]
pub struct SchedulerStats {
    pub pending_tasks: usize,
    pub running_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub timeout_tasks: usize,
    pub total_workers: usize,
    pub idle_workers: usize,
    pub busy_workers: usize,
}

/// Task scheduler
pub struct TaskScheduler {
    /// Task queue sorted by priority
    task_queue: Arc<Mutex<VecDeque<QueuedTask>>>,
    /// Registry of all task states
    task_registry: Arc<RwLock<HashMap<Uuid, QueuedTask>>>,
    /// Scheduler configuration
    config: SchedulerConfig,
    /// Application state reference
    app_state: AppState,
    /// Scheduler control channel
    control_tx: mpsc::UnboundedSender<SchedulerCommand>,
    control_rx: Arc<Mutex<mpsc::UnboundedReceiver<SchedulerCommand>>>,
}

/// Scheduler commands
#[derive(Debug)]
pub enum SchedulerCommand {
    /// Add new task
    AddTask {
        task: QueuedTask,
        response_tx: tokio::sync::oneshot::Sender<Result<Uuid, SchedulerError>>,
    },
    /// Cancel task
    CancelTask {
        task_id: Uuid,
        response_tx: tokio::sync::oneshot::Sender<Result<(), SchedulerError>>,
    },
    /// Task completion notification
    TaskCompleted {
        task_id: Uuid,
        success: bool,
        worker_id: String,
    },
    /// Worker status update
    WorkerStatusUpdate {
        worker_id: String,
        status: TaskState,
    },
    /// Get statistics
    GetStats {
        response_tx: tokio::sync::oneshot::Sender<SchedulerStats>,
    },
    /// Get task by ID
    GetTask {
        task_id: Uuid,
        response_tx: tokio::sync::oneshot::Sender<Option<QueuedTask>>,
    },
    /// Stop scheduler
    Stop,
}

/// Scheduler error types
#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("Queue is full")]
    QueueFull,
    #[error("Task not found: {0}")]
    TaskNotFound(Uuid),
    #[error("Task already exists: {0}")]
    TaskExists(Uuid),
    #[error("No available workers")]
    NoWorkers,
    #[error("Worker not found: {0}")]
    WorkerNotFound(String),
    #[error("Invalid task state transition from {from:?} to {to:?}")]
    InvalidStateTransition { from: TaskState, to: TaskState },
    #[error("Internal error: {0}")]
    Internal(String),
}

impl TaskScheduler {
    /// Create new task scheduler
    pub fn new(config: SchedulerConfig, app_state: AppState) -> Self {
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        
        Self {
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            task_registry: Arc::new(RwLock::new(HashMap::new())),
            config,
            app_state,
            control_tx,
            control_rx: Arc::new(Mutex::new(control_rx)),
        }
    }

    /// Get scheduler control handle
    pub fn get_handle(&self) -> SchedulerHandle {
        SchedulerHandle {
            control_tx: self.control_tx.clone(),
        }
    }

    /// Start scheduler
    pub async fn run(&self) {
        let scheduler_task = self.run_scheduler_loop();
        let cleanup_task = self.run_cleanup_loop();
        
        tokio::select! {
            _ = scheduler_task => {
                tracing::info!("Scheduler loop ended");
            }
            _ = cleanup_task => {
                tracing::info!("Cleanup loop ended");
            }
        }
    }

    /// Main scheduler loop
    async fn run_scheduler_loop(&self) {
        let mut interval = time::interval(self.config.scheduler_interval);
        let mut control_rx = self.control_rx.lock().await;
        
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.process_queue().await {
                        tracing::error!("Error processing queue: {}", e);
                    }
                    
                    if let Err(e) = self.check_timeouts().await {
                        tracing::error!("Error checking timeouts: {}", e);
                    }
                }
                
                command = control_rx.recv() => {
                    match command {
                        Some(cmd) => {
                            if let SchedulerCommand::Stop = cmd {
                                tracing::info!("Scheduler stopping");
                                break;
                            }
                            self.handle_command(cmd).await;
                        }
                        None => {
                            tracing::info!("Control channel closed, stopping scheduler");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Process tasks in the queue
    async fn process_queue(&self) -> Result<(), SchedulerError> {
        let mut queue = self.task_queue.lock().await;
        
        // Get available workers
        let idle_workers: Vec<String> = self.app_state
            .workers
            .iter()
            .filter(|entry| matches!(entry.value().status, WorkerStatus::Idle))
            .map(|entry| entry.key().clone())
            .collect();
        
        if idle_workers.is_empty() {
            return Ok(());
        }

        // Try to assign tasks to available workers
        let mut assigned_count = 0;
        let mut worker_iter = idle_workers.iter().cycle();
        
        while assigned_count < idle_workers.len() && !queue.is_empty() {
            if let Some(mut task) = queue.pop_front() {
                if matches!(task.state, TaskState::Pending) {
                    if let Some(worker_id) = worker_iter.next() {
                        // Assign task to worker
                        match self.assign_task_to_worker(&mut task, worker_id.clone()).await {
                            Ok(()) => {
                                // Update task state
                                let mut registry = self.task_registry.write().await;
                                registry.insert(task.task_id, task);
                                assigned_count += 1;
                            }
                            Err(e) => {
                                tracing::error!("Failed to assign task {} to worker {}: {}", 
                                    task.task_id, worker_id, e);
                                // Put task back in queue
                                queue.push_front(task);
                                break;
                            }
                        }
                    }
                } else {
                    // Task state is not Pending, put back in queue
                    queue.push_back(task);
                }
            }
        }
        
        Ok(())
    }

    /// Assign task to specified worker
    async fn assign_task_to_worker(&self, task: &mut QueuedTask, worker_id: String) -> Result<(), SchedulerError> {
        // Create WebSocket message
        let msg = WSMessage::Task {
            id: task.task_id.to_string(),
            repo: task.build_request.repo.clone(),
            target: task.target.clone(),
            args: task.build_request.args.clone(),
            mr: task.build_request.mr.clone().unwrap_or_default(),
        };

        // Send task to worker
        if let Some(mut worker) = self.app_state.workers.get_mut(&worker_id) {
            if worker.sender.send(msg).is_ok() {
                // Update worker status
                worker.status = WorkerStatus::Busy(task.task_id.to_string());
                
                // Update task state
                task.state = TaskState::Assigned(worker_id.clone());
                task.assigned_worker = Some(worker_id.clone());
                
                tracing::info!("Task {} assigned to worker {}", task.task_id, worker_id);
                Ok(())
            } else {
                Err(SchedulerError::Internal(format!("Failed to send task to worker {}", worker_id)))
            }
        } else {
            Err(SchedulerError::Internal(format!("Worker {} not found", worker_id)))
        }
    }

    /// Check for timed out tasks
    async fn check_timeouts(&self) -> Result<(), SchedulerError> {
        let now = Instant::now();
        let mut registry = self.task_registry.write().await;
        let mut timed_out_tasks = Vec::new();
        
        for (task_id, task) in registry.iter() {
            if matches!(task.state, TaskState::Assigned(_) | TaskState::Running(_)) {
                if now.duration_since(task.created_at) > task.timeout_duration {
                    timed_out_tasks.push(*task_id);
                }
            }
        }
        
        for task_id in timed_out_tasks {
            if let Some(task) = registry.get_mut(&task_id) {
                tracing::warn!("Task {} timed out", task_id);
                
                // Release worker
                if let Some(worker_id) = &task.assigned_worker {
                    if let Some(mut worker) = self.app_state.workers.get_mut(worker_id) {
                        worker.status = WorkerStatus::Idle;
                    }
                }
                
                // Check if task can be retried
                if task.retry_count < task.max_retries {
                    task.retry_count += 1;
                    task.state = TaskState::Pending;
                    task.assigned_worker = None;
                    task.created_at = now; // Reset creation time
                    
                    // Re-add to queue
                    let mut queue = self.task_queue.lock().await;
                    self.insert_task_by_priority(&mut queue, task.clone());
                    
                    tracing::info!("Task {} requeued for retry ({}/{})", 
                        task_id, task.retry_count, task.max_retries);
                } else {
                    task.state = TaskState::Timeout;
                    tracing::error!("Task {} exceeded max retries and timed out", task_id);
                }
            }
        }
        
        Ok(())
    }

    /// Insert task to queue by priority
    fn insert_task_by_priority(&self, queue: &mut VecDeque<QueuedTask>, task: QueuedTask) {
        let position = queue
            .iter()
            .position(|existing_task| existing_task.priority < task.priority)
            .unwrap_or(queue.len());
        
        queue.insert(position, task);
    }

    /// Handle scheduler commands
    async fn handle_command(&self, command: SchedulerCommand) {
        match command {
            SchedulerCommand::AddTask { task, response_tx } => {
                let result = self.add_task_internal(task).await;
                let _ = response_tx.send(result);
            }
            SchedulerCommand::CancelTask { task_id, response_tx } => {
                let result = self.cancel_task_internal(task_id).await;
                let _ = response_tx.send(result);
            }
            SchedulerCommand::TaskCompleted { task_id, success, worker_id } => {
                self.handle_task_completion(task_id, success, worker_id).await;
            }
            SchedulerCommand::WorkerStatusUpdate { worker_id, status } => {
                self.handle_worker_status_update(worker_id, status).await;
            }
            SchedulerCommand::GetStats { response_tx } => {
                let stats = self.get_stats().await;
                let _ = response_tx.send(stats);
            }
            SchedulerCommand::GetTask { task_id, response_tx } => {
                let task = self.get_task_internal(task_id).await;
                let _ = response_tx.send(task);
            }
            SchedulerCommand::Stop => {
                // Already handled in main loop
            }
        }
    }

    /// Internal add task logic
    async fn add_task_internal(&self, task: QueuedTask) -> Result<Uuid, SchedulerError> {
        let mut queue = self.task_queue.lock().await;
        
        // Check if queue is full
        if queue.len() >= self.config.max_queue_length {
            return Err(SchedulerError::QueueFull);
        }
        
        // Check if task already exists
        let registry = self.task_registry.read().await;
        if registry.contains_key(&task.task_id) {
            return Err(SchedulerError::TaskExists(task.task_id));
        }
        drop(registry);
        
        let task_id = task.task_id;
        
        // Add to registry
        let mut registry = self.task_registry.write().await;
        registry.insert(task_id, task.clone());
        drop(registry);
        
        // Insert by priority
        self.insert_task_by_priority(&mut queue, task);
        
        tracing::info!("Task {} added to queue", task_id);
        Ok(task_id)
    }

    /// Internal cancel task logic
    async fn cancel_task_internal(&self, task_id: Uuid) -> Result<(), SchedulerError> {
        let mut registry = self.task_registry.write().await;
        
        if let Some(task) = registry.get_mut(&task_id) {
            match &task.state {
                TaskState::Pending => {
                    // Remove from queue
                    let mut queue = self.task_queue.lock().await;
                    queue.retain(|t| t.task_id != task_id);
                    task.state = TaskState::Cancelled;
                }
                TaskState::Assigned(worker_id) | TaskState::Running(worker_id) => {
                    // Notify worker to cancel task (if needed)
                    if let Some(mut worker) = self.app_state.workers.get_mut(worker_id) {
                        worker.status = WorkerStatus::Idle;
                    }
                    task.state = TaskState::Cancelled;
                }
                _ => {
                    return Err(SchedulerError::Internal("Cannot cancel completed task".to_string()));
                }
            }
            
            tracing::info!("Task {} cancelled", task_id);
            Ok(())
        } else {
            Err(SchedulerError::TaskNotFound(task_id))
        }
    }

    /// Handle task completion
    async fn handle_task_completion(&self, task_id: Uuid, success: bool, worker_id: String) {
        let mut registry = self.task_registry.write().await;
        
        if let Some(task) = registry.get_mut(&task_id) {
            task.state = if success {
                TaskState::Completed
            } else {
                TaskState::Failed
            };
            
            // Release worker
            if let Some(mut worker) = self.app_state.workers.get_mut(&worker_id) {
                worker.status = WorkerStatus::Idle;
            }
            
            tracing::info!("Task {} completed with success: {}", task_id, success);
        }
    }

    /// Handle worker status update
    async fn handle_worker_status_update(&self, worker_id: String, status: TaskState) {
        if let Some(mut worker) = self.app_state.workers.get_mut(&worker_id) {
            match status {
                TaskState::Running(task_id) => {
                    worker.status = WorkerStatus::Busy(task_id.clone());
                    // Update task state in registry
                    let mut registry = self.task_registry.write().await;
                    if let Some(task) = registry.get_mut(&task_id.parse::<Uuid>().unwrap_or_default()) {
                        task.state = TaskState::Running(worker_id);
                    }
                }
                _ => {
                    // Other status updates can be handled here
                    tracing::debug!("Worker {} status updated to {:?}", worker_id, status);
                }
            }
        }
    }

    /// Get task by ID (internal method)
    async fn get_task_internal(&self, task_id: Uuid) -> Option<QueuedTask> {
        let registry = self.task_registry.read().await;
        registry.get(&task_id).cloned()
    }

    /// Get statistics
    async fn get_stats(&self) -> SchedulerStats {
        let registry = self.task_registry.read().await;
        
        let mut stats = SchedulerStats {
            pending_tasks: 0,
            running_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            timeout_tasks: 0,
            total_workers: self.app_state.workers.len(),
            idle_workers: 0,
            busy_workers: 0,
        };
        
        // Count task states
        for task in registry.values() {
            match task.state {
                TaskState::Pending => stats.pending_tasks += 1,
                TaskState::Assigned(_) | TaskState::Running(_) => stats.running_tasks += 1,
                TaskState::Completed => stats.completed_tasks += 1,
                TaskState::Failed => stats.failed_tasks += 1,
                TaskState::Timeout => stats.timeout_tasks += 1,
                TaskState::Cancelled => {} // Not counted in statistics
            }
        }
        
        // Count worker states
        for worker in self.app_state.workers.iter() {
            match worker.value().status {
                WorkerStatus::Idle => stats.idle_workers += 1,
                WorkerStatus::Busy(_) => stats.busy_workers += 1,
            }
        }
        
        stats
    }

    /// Cleanup loop
    async fn run_cleanup_loop(&self) {
        let mut interval = time::interval(self.config.cleanup_interval);
        
        loop {
            interval.tick().await;
            self.cleanup_completed_tasks().await;
        }
    }

    /// Clean up completed tasks
    async fn cleanup_completed_tasks(&self) {
        let now = Instant::now();
        let retention = self.config.completed_task_retention;
        
        let mut registry = self.task_registry.write().await;
        let mut to_remove = Vec::new();
        
        for (task_id, task) in registry.iter() {
            if matches!(task.state, TaskState::Completed | TaskState::Failed | TaskState::Timeout | TaskState::Cancelled) {
                if now.duration_since(task.created_at) > retention {
                    to_remove.push(*task_id);
                }
            }
        }
        
        for task_id in to_remove {
            registry.remove(&task_id);
            tracing::debug!("Cleaned up completed task {}", task_id);
        }
    }
}

/// Scheduler control handle
#[derive(Clone)]
pub struct SchedulerHandle {
    control_tx: mpsc::UnboundedSender<SchedulerCommand>,
}

impl SchedulerHandle {
    /// Add task to queue
    pub async fn add_task(&self, task: QueuedTask) -> Result<Uuid, SchedulerError> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        self.control_tx.send(SchedulerCommand::AddTask {
            task,
            response_tx,
        }).map_err(|_| SchedulerError::Internal("Scheduler not running".to_string()))?;
        
        response_rx.await
            .map_err(|_| SchedulerError::Internal("Response channel closed".to_string()))?
    }

    /// Cancel task
    pub async fn cancel_task(&self, task_id: Uuid) -> Result<(), SchedulerError> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        self.control_tx.send(SchedulerCommand::CancelTask {
            task_id,
            response_tx,
        }).map_err(|_| SchedulerError::Internal("Scheduler not running".to_string()))?;
        
        response_rx.await
            .map_err(|_| SchedulerError::Internal("Response channel closed".to_string()))?
    }

    /// Notify task completed
    pub async fn notify_task_completed(&self, task_id: Uuid, success: bool, worker_id: String) -> Result<(), SchedulerError> {
        self.control_tx.send(SchedulerCommand::TaskCompleted {
            task_id,
            success,
            worker_id,
        }).map_err(|_| SchedulerError::Internal("Scheduler not running".to_string()))?;
        
        Ok(())
    }

    /// Update worker status
    pub async fn update_worker_status(&self, worker_id: String, status: TaskState) -> Result<(), SchedulerError> {
        self.control_tx.send(SchedulerCommand::WorkerStatusUpdate {
            worker_id,
            status,
        }).map_err(|_| SchedulerError::Internal("Scheduler not running".to_string()))?;
        
        Ok(())
    }

    /// Get task by ID
    pub async fn get_task(&self, task_id: Uuid) -> Result<Option<QueuedTask>, SchedulerError> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        self.control_tx.send(SchedulerCommand::GetTask {
            task_id,
            response_tx,
        }).map_err(|_| SchedulerError::Internal("Scheduler not running".to_string()))?;
        
        response_rx.await
            .map_err(|_| SchedulerError::Internal("Response channel closed".to_string()))
    }

    /// Get statistics
    pub async fn get_stats(&self) -> Result<SchedulerStats, SchedulerError> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        self.control_tx.send(SchedulerCommand::GetStats {
            response_tx,
        }).map_err(|_| SchedulerError::Internal("Scheduler not running".to_string()))?;
        
        response_rx.await
            .map_err(|_| SchedulerError::Internal("Response channel closed".to_string()))
    }

    /// Stop scheduler
    pub async fn stop(&self) -> Result<(), SchedulerError> {
        self.control_tx.send(SchedulerCommand::Stop)
            .map_err(|_| SchedulerError::Internal("Scheduler not running".to_string()))?;
        
        Ok(())
    }
}
