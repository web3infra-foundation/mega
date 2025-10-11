# Task Scheduler Refactoring

This document describes the refactored task scheduling system that replaces the simple direct worker assignment with a sophisticated queue-based scheduler.

## Overview

The original `task_handler` function had several limitations:
- Direct worker assignment without queuing
- No retry mechanism for failed tasks
- No task prioritization
- No timeout handling
- Poor resource utilization when workers are busy

The new scheduler addresses these issues with:
- **Queue-based task management** with priority support
- **Automatic timeout detection and retry logic**
- **Worker load balancing** across multiple nodes
- **Task state tracking** and monitoring
- **Graceful degradation** when resources are constrained

## Architecture

### Components

1. **TaskScheduler**: Core scheduler managing task queues and worker assignment
2. **QueuedTask**: Task representation with state, priority, and retry information
3. **SchedulerHandle**: Thread-safe handle for interacting with the scheduler
4. **SchedulerConfig**: Configuration for timeouts, retries, and queue limits

### Task States

Tasks progress through the following states:
- `Pending`: Waiting in queue for assignment
- `Assigned`: Sent to a worker but not yet running
- `Running`: Currently executing on a worker
- `Completed`: Successfully finished
- `Failed`: Execution failed
- `Timeout`: Exceeded time limit
- `Cancelled`: Manually cancelled

### Priority Levels

Tasks can have different priority levels:
- `Critical`: Hotfixes, urgent production issues
- `High`: Important features, releases, core repositories
- `Normal`: Standard development tasks
- `Low`: Documentation, refactoring, non-critical work

## Key Features

### 1. Queue Management

```rust
// Tasks are queued with priority ordering
let task = QueuedTask::new(build_request, target, &config, Some(TaskPriority::High));
scheduler_handle.add_task(task).await?;
```

### 2. Automatic Retry Logic

```rust
// Tasks are automatically retried on failure/timeout
if task.retry_count < task.max_retries {
    task.reset_for_retry();
    // Re-queue for execution
}
```

### 3. Worker Load Balancing

The scheduler distributes tasks across available workers using round-robin assignment, ensuring even workload distribution.

### 4. Timeout Handling

Tasks that exceed their timeout are automatically:
- Removed from workers
- Marked as timed out
- Retried if retry count allows
- Cleaned up if max retries exceeded

### 5. Statistics and Monitoring

```rust
let stats = scheduler_handle.get_stats().await?;
// Returns: pending_tasks, running_tasks, completed_tasks, etc.
```

## Usage Examples

### Basic Task Submission

```rust
// Create and submit a task
let config = SchedulerConfig::default();
let task = QueuedTask::new(build_request, target, &config, Some(TaskPriority::Normal));
let task_id = scheduler_handle.add_task(task).await?;
```

### Task Cancellation

```rust
// Cancel a running or queued task
scheduler_handle.cancel_task(task_id).await?;
```

### Getting Task Status

```rust
// Get detailed task information
let task = scheduler_handle.get_task(task_id).await?;
```

### System Monitoring

```rust
// Monitor scheduler performance
let stats = scheduler_handle.get_stats().await?;
println!("Pending: {}, Running: {}, Workers: {}", 
         stats.pending_tasks, stats.running_tasks, stats.total_workers);
```

## Configuration

### Scheduler Configuration

```rust
let config = SchedulerConfig {
    max_queue_length: 500,              // Maximum queued tasks
    default_task_timeout: Duration::from_secs(7200),  // 2 hours
    default_max_retries: 2,             // Retry failed tasks twice
    scheduler_interval: Duration::from_millis(500),   // Process queue every 500ms
    cleanup_interval: Duration::from_secs(300),       // Clean up every 5 minutes
    completed_task_retention: Duration::from_secs(7200), // Keep completed tasks for 2 hours
};
```

### Priority Determination

The system automatically determines task priority based on:
- **Change List labels**: `hotfix`, `urgent`, `critical` → Critical priority
- **Repository importance**: `core`, `main`, `production` → High priority
- **Build complexity**: Fewer arguments → Higher priority (faster feedback)
- **System load**: High load → Lower priority for complex tasks

## API Endpoints

### New Scheduler Endpoints

- `POST /scheduler/task` - Submit task to scheduler
- `GET /scheduler/task/{id}` - Get task status and details
- `DELETE /scheduler/task/{id}` - Cancel a task
- `GET /scheduler/stats` - Get scheduler statistics

### Enhanced Endpoints

- `POST /v2/task` - Enhanced task submission with scheduler integration
- `GET /v2/health` - Health check including scheduler status

## Migration Guide

### From Original System

1. **Replace direct worker assignment**:
   ```rust
   // Old: Direct assignment
   let chosen_worker = select_random_worker();
   send_task_to_worker(task, chosen_worker);
   
   // New: Queue-based scheduling
   let task = QueuedTask::new(build_request, target, &config, priority);
   scheduler_handle.add_task(task).await?;
   ```

2. **Update error handling**:
   ```rust
   // Handle scheduler-specific errors
   match scheduler_handle.add_task(task).await {
       Ok(task_id) => { /* success */ },
       Err(SchedulerError::QueueFull) => { /* queue full */ },
       Err(SchedulerError::NoWorkers) => { /* no workers */ },
       Err(e) => { /* other errors */ },
   }
   ```

3. **Initialize scheduler at startup**:
   ```rust
   // Initialize scheduler before starting server
   let scheduler_handle = initialize_scheduler(app_state).await?;
   ```

## Benefits

### Performance Improvements

- **Better resource utilization**: Tasks wait in queue instead of being rejected
- **Load balancing**: Even distribution across workers
- **Priority handling**: Critical tasks get processed first

### Reliability Improvements

- **Automatic retries**: Failed tasks are retried automatically
- **Timeout detection**: Hung tasks are detected and recovered
- **Graceful degradation**: System remains functional under high load

### Operational Improvements

- **Monitoring**: Detailed statistics for system health
- **Task tracking**: Complete task lifecycle visibility
- **Capacity planning**: Queue metrics help with scaling decisions

## Monitoring and Alerting

### Key Metrics to Monitor

- `pending_tasks`: Queue depth (alert if > threshold)
- `timeout_tasks`: Failed tasks due to timeout (investigate if increasing)
- `idle_workers`: Available capacity (scale up if consistently 0)
- `queue_full_errors`: Rejected tasks (increase queue size or workers)

### Health Check Integration

The scheduler is integrated into health checks:
- System is "healthy" if workers are available and queue is manageable
- System is "degraded" if queue is backing up
- System is "unhealthy" if scheduler is not operational

## Future Enhancements

1. **Persistent Queue**: Store queue state in database for crash recovery
2. **Worker Affinity**: Assign tasks to specific worker types
3. **Resource Constraints**: Consider CPU/memory when scheduling
4. **Task Dependencies**: Support for task execution ordering
5. **Batch Processing**: Group related tasks for efficiency
6. **Auto-scaling**: Automatically request more workers based on queue depth

## Error Handling

The scheduler provides comprehensive error handling:
- `QueueFull`: Queue has reached maximum capacity
- `TaskNotFound`: Requested task doesn't exist
- `TaskExists`: Attempting to add duplicate task
- `NoWorkers`: No workers available for assignment
- `WorkerNotFound`: Assigned worker is no longer available
- `InvalidStateTransition`: Illegal task state change
- `Internal`: Other internal errors

Each error provides detailed context for debugging and user feedback.
