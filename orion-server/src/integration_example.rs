use std::sync::Arc;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tokio::sync::Mutex;
use tracing::{info, error};

use crate::api::{AppState, BuildRequest};
use crate::scheduler::{TaskScheduler, SchedulerConfig, SchedulerHandle, QueuedTask, TaskPriority};
use crate::buck2::download_and_get_buck2_targets;

/// Integration example showing how to set up and use the scheduler
/// This demonstrates the complete workflow from setup to task execution

/// Global scheduler handle - in a real application, this would be managed differently
static SCHEDULER_HANDLE: once_cell::sync::OnceCell<SchedulerHandle> = once_cell::sync::OnceCell::new();

/// Initialize the scheduler system
pub async fn initialize_scheduler(app_state: AppState) -> Result<SchedulerHandle, Box<dyn std::error::Error>> {
    info!("Initializing task scheduler...");
    
    // Create scheduler configuration with production-ready settings
    let config = SchedulerConfig {
        max_queue_length: 500,
        default_task_timeout: std::time::Duration::from_secs(7200), // 2 hours
        default_max_retries: 2,
        scheduler_interval: std::time::Duration::from_millis(500),
        cleanup_interval: std::time::Duration::from_secs(300), // 5 minutes
        completed_task_retention: std::time::Duration::from_secs(7200), // 2 hours
    };

    // Create and start the scheduler
    let scheduler = TaskScheduler::new(config, app_state);
    let handle = scheduler.get_handle();
    
    // Start the scheduler in the background
    let scheduler_arc = Arc::new(scheduler);
    let scheduler_clone = scheduler_arc.clone();
    
    tokio::spawn(async move {
        scheduler_clone.run().await;
    });
    
    // Initialize the global handle
    SCHEDULER_HANDLE.set(handle.clone())
        .map_err(|_| "Failed to set global scheduler handle")?;
    
    info!("Task scheduler initialized successfully");
    Ok(handle)
}

/// Get the global scheduler handle
pub fn get_scheduler_handle() -> Option<&'static SchedulerHandle> {
    SCHEDULER_HANDLE.get()
}

/// Enhanced task handler that demonstrates the complete integration
#[utoipa::path(
    post,
    path = "/v2/task",
    request_body = BuildRequest,
    responses(
        (status = 200, description = "Task queued successfully", body = serde_json::Value),
        (status = 503, description = "Service unavailable", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    )
)]
pub async fn enhanced_task_handler(
    State(app_state): State<AppState>,
    Json(req): Json<BuildRequest>,
) -> impl IntoResponse {
    info!("Received task request for repo: {}", req.repo);
    
    // Get the scheduler handle
    let scheduler_handle = match get_scheduler_handle() {
        Some(handle) => handle,
        None => {
            error!("Scheduler not initialized");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Scheduler not available",
                    "message": "Task scheduler is not initialized"
                })),
            ).into_response();
        }
    };

    // Check if we have any available workers before proceeding
    match scheduler_handle.get_stats().await {
        Ok(stats) => {
            if stats.total_workers == 0 {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({
                        "error": "No workers available",
                        "message": "No worker nodes are currently connected"
                    })),
                ).into_response();
            }
            
            if stats.idle_workers == 0 && stats.pending_tasks > 10 {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({
                        "error": "System busy",
                        "message": "All workers are busy and queue is getting full",
                        "stats": {
                            "pending_tasks": stats.pending_tasks,
                            "running_tasks": stats.running_tasks,
                            "idle_workers": stats.idle_workers,
                            "busy_workers": stats.busy_workers
                        }
                    })),
                ).into_response();
            }
        }
        Err(e) => {
            error!("Failed to get scheduler stats: {}", e);
        }
    }

    // Download and get buck2 targets
    let target = match download_and_get_buck2_targets(&req.buck_hash, &req.buckconfig_hash).await {
        Ok(target) => target,
        Err(e) => {
            error!("Failed to download buck2 targets: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to prepare build environment",
                    "message": format!("Buck2 target resolution failed: {}", e)
                })),
            ).into_response();
        }
    };

    // Determine task priority based on various factors
    let priority = determine_priority(&req, &app_state).await;
    
    // Create scheduler configuration (this could be cached)
    let config = SchedulerConfig::default();
    
    // Create queued task
    let task = QueuedTask::new(req.clone(), target, &config, Some(priority));
    let task_id = task.task_id;
    
    // Submit task to scheduler
    match scheduler_handle.add_task(task).await {
        Ok(queued_task_id) => {
            info!("Task {} successfully queued with priority {:?}", queued_task_id, priority);
            
            // Return enhanced response with useful information
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "task_id": queued_task_id.to_string(),
                    "status": "queued",
                    "priority": format!("{:?}", priority),
                    "message": "Task has been queued for execution",
                    "endpoints": {
                        "status": format!("/v2/task/{}/status", queued_task_id),
                        "cancel": format!("/v2/task/{}/cancel", queued_task_id),
                        "output": format!("/task-output/{}", queued_task_id)
                    }
                })),
            ).into_response()
        }
        Err(crate::scheduler::SchedulerError::QueueFull) => {
            error!("Queue is full, rejecting task for repo: {}", req.repo);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Queue is full",
                    "message": "The task queue is currently at capacity. Please try again later.",
                    "retry_after": 60
                })),
            ).into_response()
        }
        Err(crate::scheduler::SchedulerError::TaskExists(existing_id)) => {
            error!("Duplicate task ID {}", existing_id);
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "Task already exists",
                    "task_id": existing_id.to_string(),
                    "message": "A task with this ID already exists in the system"
                })),
            ).into_response()
        }
        Err(e) => {
            error!("Failed to queue task {}: {}", task_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to queue task",
                    "message": "An internal error occurred while queueing the task"
                })),
            ).into_response()
        }
    }
}

/// Advanced priority determination logic
async fn determine_priority(req: &BuildRequest, app_state: &AppState) -> TaskPriority {
    // Check MR-based priority
    if let Some(mr) = &req.mr {
        let mr_lower = mr.to_lowercase();
        
        // Critical priority for hotfixes and urgent changes
        if mr_lower.contains("hotfix") || mr_lower.contains("urgent") || mr_lower.contains("critical") {
            return TaskPriority::Critical;
        }
        
        // High priority for releases and important features
        if mr_lower.contains("release") || mr_lower.contains("feature") {
            return TaskPriority::High;
        }
        
        // Low priority for documentation and minor changes
        if mr_lower.contains("docs") || mr_lower.contains("refactor") || mr_lower.contains("minor") {
            return TaskPriority::Low;
        }
    }
    
    // Check repository importance (you could maintain a config for this)
    if req.repo.contains("core") || req.repo.contains("main") || req.repo.contains("production") {
        return TaskPriority::High;
    }
    
    // Check build complexity - simpler builds get higher priority for faster feedback
    if let Some(args) = &req.args {
        if args.len() <= 2 {
            return TaskPriority::High;
        }
        if args.len() > 5 {
            return TaskPriority::Low;
        }
    }
    
    // Check current system load
    if let Some(handle) = get_scheduler_handle() {
        if let Ok(stats) = handle.get_stats().await {
            // If system is under load, prioritize smaller/faster tasks
            if stats.pending_tasks > 20 && stats.idle_workers < 2 {
                return TaskPriority::Low;
            }
        }
    }
    
    TaskPriority::Normal
}

/// Health check endpoint that includes scheduler status
#[utoipa::path(
    get,
    path = "/v2/health",
    responses(
        (status = 200, description = "System health status", body = serde_json::Value),
        (status = 503, description = "System unhealthy", body = serde_json::Value)
    )
)]
pub async fn health_check_with_scheduler() -> impl IntoResponse {
    let mut health_status = serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION")
    });
    
    // Add scheduler health information
    if let Some(handle) = get_scheduler_handle() {
        match handle.get_stats().await {
            Ok(stats) => {
                health_status["scheduler"] = serde_json::json!({
                    "status": "operational",
                    "stats": stats
                });
                
                // Determine if system is healthy based on scheduler stats
                let is_healthy = stats.total_workers > 0 && 
                                stats.pending_tasks < stats.total_workers * 10;
                
                if !is_healthy {
                    health_status["status"] = serde_json::Value::String("degraded".to_string());
                }
            }
            Err(e) => {
                error!("Failed to get scheduler stats for health check: {}", e);
                health_status["scheduler"] = serde_json::json!({
                    "status": "error",
                    "error": format!("{}", e)
                });
                health_status["status"] = serde_json::Value::String("unhealthy".to_string());
            }
        }
    } else {
        health_status["scheduler"] = serde_json::json!({
            "status": "not_initialized"
        });
        health_status["status"] = serde_json::Value::String("unhealthy".to_string());
    }
    
    let status_code = match health_status["status"].as_str() {
        Some("healthy") => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    
    (status_code, Json(health_status)).into_response()
}

/// Example of how to integrate scheduler into your main application
pub async fn setup_application_with_scheduler(app_state: AppState) -> Result<axum::Router, Box<dyn std::error::Error>> {
    // Initialize the scheduler
    let _scheduler_handle = initialize_scheduler(app_state.clone()).await?;
    
    // Create router with enhanced endpoints
    let app = axum::Router::new()
        .route("/v2/task", axum::routing::post(enhanced_task_handler))
        .route("/v2/health", axum::routing::get(health_check_with_scheduler))
        // Add scheduler management routes
        .merge(crate::api_scheduler::scheduler_routes())
        // Include original routes for backward compatibility
        .merge(crate::api::routers())
        .with_state(app_state);
    
    Ok(app)
}
