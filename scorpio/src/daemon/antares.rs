//! Antares daemon HTTP interface for mount lifecycle management.
//!
//! Provides Axum routes to create, list, query, and delete FUSE mounts backed by
//! AntaresService implementations. Includes graceful shutdown with cleanup.

use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{sync::RwLock, time::timeout};
use uuid::Uuid;

use crate::{
    antares::fuse::AntaresFuse,
    dicfuse::{Dicfuse, DicfuseManager},
};

/// High-level HTTP daemon that exposes Antares orchestration capabilities.
pub struct AntaresDaemon<S: AntaresService> {
    service: Arc<S>,
    shutdown_timeout: Duration,
}

impl<S> AntaresDaemon<S>
where
    S: AntaresService + 'static,
{
    /// Construct a daemon backed by the given service.
    pub fn new(service: Arc<S>) -> Self {
        Self {
            service,
            shutdown_timeout: Duration::from_secs(10),
        }
    }

    /// Override the graceful shutdown timeout applied to the HTTP server.
    pub fn with_shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Produce an Axum router with all routes wired to their handlers.
    pub fn router(&self) -> Router {
        Router::new()
            .route("/health", get(Self::healthcheck))
            .route("/mounts", post(Self::create_mount))
            .route("/mounts", get(Self::list_mounts))
            .route("/mounts/by-job/{job_id}", get(Self::describe_mount_by_job))
            .route("/mounts/by-job/{job_id}", delete(Self::delete_mount_by_job))
            .route("/mounts/{mount_id}", get(Self::describe_mount))
            .route("/mounts/{mount_id}", delete(Self::delete_mount))
            .route("/mounts/{mount_id}/cl", post(Self::build_cl))
            .route("/mounts/{mount_id}/cl", delete(Self::clear_cl))
            .with_state(self.service.clone())
    }

    /// Run the HTTP server until it receives a shutdown signal.
    /// Note: For graceful shutdown with mount cleanup, use AntaresDaemon<AntaresServiceImpl>.
    pub async fn serve(self, bind_addr: SocketAddr) -> Result<(), ApiError> {
        let router = self.router();
        let shutdown_timeout = self.shutdown_timeout;
        let service = self.service.clone();

        let listener = tokio::net::TcpListener::bind(bind_addr)
            .await
            .map_err(|e| {
                ApiError::Service(ServiceError::Internal(format!(
                    "failed to bind to {}: {}",
                    bind_addr, e
                )))
            })?;

        tracing::info!("Antares daemon listening on {}", bind_addr);

        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                tracing::info!("Received shutdown signal");
                match timeout(shutdown_timeout, service.shutdown_cleanup()).await {
                    Ok(Ok(())) => tracing::info!("Shutdown cleanup completed"),
                    Ok(Err(e)) => tracing::warn!("Shutdown cleanup failed: {:?}", e),
                    Err(_) => {
                        tracing::warn!("Shutdown cleanup timed out after {:?}", shutdown_timeout)
                    }
                }
            })
            .await
            .map_err(|e| {
                ApiError::Service(ServiceError::Internal(format!("server error: {}", e)))
            })?;

        Ok(())
    }

    /// Lightweight health/liveness probe.
    async fn healthcheck(State(service): State<Arc<S>>) -> Result<Json<HealthResponse>, ApiError> {
        Ok(Json(service.health_info().await))
    }

    async fn create_mount(
        State(service): State<Arc<S>>,
        Json(request): Json<CreateMountRequest>,
    ) -> Result<Json<MountCreated>, ApiError> {
        let created = service.create_mount(request).await?;
        Ok(Json(created))
    }

    async fn list_mounts(State(service): State<Arc<S>>) -> Result<Json<MountCollection>, ApiError> {
        let mounts = service.list_mounts().await?;
        Ok(Json(MountCollection { mounts }))
    }

    async fn describe_mount_by_job(
        State(service): State<Arc<S>>,
        Path(job_id): Path<String>,
    ) -> Result<Json<MountStatus>, ApiError> {
        let status = service.describe_mount_by_job(job_id).await?;
        Ok(Json(status))
    }

    async fn delete_mount_by_job(
        State(service): State<Arc<S>>,
        Path(job_id): Path<String>,
    ) -> Result<Json<MountStatus>, ApiError> {
        let status = service.delete_mount_by_job(job_id).await?;
        Ok(Json(status))
    }

    async fn describe_mount(
        State(service): State<Arc<S>>,
        Path(mount_id): Path<Uuid>,
    ) -> Result<Json<MountStatus>, ApiError> {
        let status = service.describe_mount(mount_id).await?;
        Ok(Json(status))
    }

    async fn delete_mount(
        State(service): State<Arc<S>>,
        Path(mount_id): Path<Uuid>,
    ) -> Result<Json<MountStatus>, ApiError> {
        let status = service.delete_mount(mount_id).await?;
        Ok(Json(status))
    }

    async fn build_cl(
        State(service): State<Arc<S>>,
        Path(mount_id): Path<Uuid>,
        Json(request): Json<BuildClRequest>,
    ) -> Result<Json<MountStatus>, ApiError> {
        let status = service.build_cl(mount_id, request.cl).await?;
        Ok(Json(status))
    }

    async fn clear_cl(
        State(service): State<Arc<S>>,
        Path(mount_id): Path<Uuid>,
    ) -> Result<Json<MountStatus>, ApiError> {
        let status = service.clear_cl(mount_id).await?;
        Ok(Json(status))
    }
}

/// Asynchronous service boundary that the HTTP layer depends on.
#[async_trait]
pub trait AntaresService: Send + Sync {
    /// Create a new mount with auto-generated paths based on UUID
    async fn create_mount(&self, request: CreateMountRequest)
        -> Result<MountCreated, ServiceError>;
    async fn list_mounts(&self) -> Result<Vec<MountStatus>, ServiceError>;
    async fn describe_mount(&self, mount_id: Uuid) -> Result<MountStatus, ServiceError>;
    async fn delete_mount(&self, mount_id: Uuid) -> Result<MountStatus, ServiceError>;

    /// Describe a mount by build task identifier (job/build id).
    ///
    /// Default implementation scans `list_mounts()`; implementations may override
    /// for efficiency.
    async fn describe_mount_by_job(&self, job_id: String) -> Result<MountStatus, ServiceError> {
        let mounts = self.list_mounts().await?;
        mounts
            .into_iter()
            .find(|m| m.job_id.as_deref() == Some(job_id.as_str()))
            .ok_or(ServiceError::NotFoundTask(job_id))
    }

    /// Delete (unmount) a mount by build task identifier (job/build id).
    ///
    /// Default implementation resolves to a mount_id and delegates to `delete_mount()`.
    async fn delete_mount_by_job(&self, job_id: String) -> Result<MountStatus, ServiceError> {
        let status = self.describe_mount_by_job(job_id.clone()).await?;
        self.delete_mount(status.mount_id).await
    }
    /// Build or rebuild the CL layer for an existing mount
    async fn build_cl(&self, mount_id: Uuid, cl_link: String) -> Result<MountStatus, ServiceError>;
    /// Clear the CL layer for an existing mount
    async fn clear_cl(&self, mount_id: Uuid) -> Result<MountStatus, ServiceError>;
    async fn health_info(&self) -> HealthResponse;
    async fn shutdown_cleanup(&self) -> Result<(), ServiceError>;
}

/// Request payload for provisioning a new mount.
/// Simplified API: only requires the monorepo path and optional CL identifier.
/// All internal paths (mountpoint, upper_dir, cl_dir) are auto-generated.
///
/// # Path Generation
/// Paths are auto-generated using UUID-based naming under configured root directories:
/// - `mountpoint`: `{antares_mount_root}/{uuid}` (e.g., `/var/lib/antares/mounts/550e8400-e29b-41d4-a716-446655440000`)
/// - `upper_dir`: `{antares_upper_root}/{uuid}` (e.g., `/var/lib/antares/upper/550e8400-e29b-41d4-a716-446655440000`)
/// - `cl_dir`: `{antares_cl_root}/{uuid}` (only if `cl` is provided)
///
/// The UUID is generated per mount request, ensuring unique paths for each mount instance.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateMountRequest {
    /// Optional build task identifier (job-level mount). When provided, Antares will treat
    /// mount creation as idempotent for the same task id.
    ///
    /// This is preferred in build systems to bind mount lifecycle to a task.
    #[serde(default)]
    pub job_id: Option<String>,
    /// Optional alternative task identifier (build-level). If both `job_id` and `build_id`
    /// are provided, `job_id` takes precedence.
    #[serde(default)]
    pub build_id: Option<String>,
    /// Monorepo path to mount (e.g., "/third-party/mega")
    pub path: String,
    /// Optional CL (changelist) identifier for the CL layer
    #[serde(default)]
    pub cl: Option<String>,
}

/// Request payload for building/rebuilding a CL layer.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildClRequest {
    /// CL (changelist) link identifier
    pub cl: String,
}

/// Response returned after mount creation succeeds.
/// Only contains the essential information the caller needs.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MountCreated {
    /// Unique identifier for this mount
    pub mount_id: Uuid,
    /// The actual filesystem path where the mount is accessible
    pub mountpoint: String,
}

/// Snapshot of a single mount's state.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MountStatus {
    pub mount_id: Uuid,
    /// Optional build task identifier (job/build id) associated with this mount.
    #[serde(default)]
    pub job_id: Option<String>,
    /// The monorepo path being mounted
    pub path: String,
    /// Optional CL identifier
    pub cl: Option<String>,
    /// The actual filesystem mountpoint
    pub mountpoint: String,
    pub layers: MountLayers,
    pub state: MountLifecycle,
    pub created_at_epoch_ms: u64,
    pub last_seen_epoch_ms: u64,
}

/// Convenience wrapper used by list endpoints.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MountCollection {
    pub mounts: Vec<MountStatus>,
}

/// Directory layout for a mount.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MountLayers {
    pub upper: String,
    pub cl: Option<String>,
    pub dicfuse: String,
}

/// Lifecycle indicator used in responses and service contracts.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum MountLifecycle {
    Provisioning,
    Mounted,
    Unmounting,
    Unmounted,
    Failed { reason: String },
}

/// Health check response payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthResponse {
    /// Service health status: "healthy" or "degraded"
    pub status: String,
    /// Current number of active mounts
    pub mount_count: usize,
    /// Service uptime in seconds
    pub uptime_secs: u64,
}

/// Error response body for JSON output.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ErrorBody {
    /// Human-readable error message
    pub error: String,
    /// Machine-readable error code
    pub code: String,
}

/// Service-level failures (implementation specific) that surface through the API.
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("mount not found: {0}")]
    NotFound(Uuid),
    #[error("mount not found for task id: {0}")]
    NotFoundTask(String),
    #[error("failed to interact with fuse stack: {0}")]
    FuseFailure(String),
    #[error("unexpected error: {0}")]
    Internal(String),
}

/// HTTP-facing errors mapped to responses.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error("serde payload rejected: {0}")]
    BadPayload(String),
    #[error("server shutting down")]
    Shutdown,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status_code, error_code, message) = match &self {
            ApiError::Service(ServiceError::InvalidRequest(msg)) => {
                (StatusCode::BAD_REQUEST, "INVALID_REQUEST", msg.clone())
            }
            ApiError::Service(ServiceError::NotFound(id)) => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                format!("mount {} not found", id),
            ),
            ApiError::Service(ServiceError::NotFoundTask(task)) => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                format!("mount for task {} not found", task),
            ),
            ApiError::Service(ServiceError::FuseFailure(msg)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "FUSE_ERROR", msg.clone())
            }
            ApiError::Service(ServiceError::Internal(msg)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
            ),
            ApiError::BadPayload(msg) => (StatusCode::BAD_REQUEST, "BAD_PAYLOAD", msg.clone()),
            ApiError::Shutdown => (
                StatusCode::SERVICE_UNAVAILABLE,
                "SHUTDOWN",
                "server is shutting down".into(),
            ),
        };

        let body = ErrorBody {
            error: message,
            code: error_code.to_string(),
        };

        (status_code, Json(body)).into_response()
    }
}

// ============================================================================
// Service Implementation
// ============================================================================

/// Internal entry tracking a single mount.
struct MountEntry {
    mount_id: Uuid,
    /// Optional build task identifier (job/build id) associated with this mount.
    job_id: Option<String>,
    /// The monorepo path being mounted
    path: String,
    /// Optional CL identifier
    cl: Option<String>,
    /// Auto-generated mountpoint path
    mountpoint: String,
    /// Auto-generated upper directory
    upper_dir: String,
    /// Auto-generated CL directory (if cl is provided)
    cl_dir: Option<String>,
    fuse: AntaresFuse,
    state: MountLifecycle,
    created_at_epoch_ms: u64,
    last_seen_epoch_ms: u64,
}

impl MountEntry {
    /// Convert to public MountStatus for API responses.
    fn to_status(&self) -> MountStatus {
        MountStatus {
            mount_id: self.mount_id,
            job_id: self.job_id.clone(),
            path: self.path.clone(),
            cl: self.cl.clone(),
            mountpoint: self.mountpoint.clone(),
            layers: MountLayers {
                upper: self.upper_dir.clone(),
                cl: self.cl_dir.clone(),
                dicfuse: "shared".to_string(),
            },
            state: self.state.clone(),
            created_at_epoch_ms: self.created_at_epoch_ms,
            last_seen_epoch_ms: self.last_seen_epoch_ms,
        }
    }

    /// Update the last_seen timestamp.
    fn update_last_seen(&mut self) {
        self.last_seen_epoch_ms = current_epoch_ms();
    }
}

/// Get current time as milliseconds since UNIX epoch.
fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Type alias for path index: maps (monorepo_path, optional_cl) to mount_id.
type PathIndex = Arc<RwLock<HashMap<(String, Option<String>), Uuid>>>;
/// Type alias for job index: maps a build task id (job_id/build_id) to mount_id.
type JobIndex = Arc<RwLock<HashMap<String, Uuid>>>;

/// Persisted mount state for recovery across restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedMountState {
    pub mount_id: Uuid,
    #[serde(default)]
    pub job_id: Option<String>,
    pub path: String,
    pub cl: Option<String>,
    pub mountpoint: String,
    pub upper_dir: String,
    pub cl_dir: Option<String>,
    pub created_at_epoch_ms: u64,
}

/// Persisted state file structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedState {
    pub mounts: Vec<PersistedMountState>,
}

/// Concrete implementation of AntaresService.
pub struct AntaresServiceImpl {
    /// Shared Dicfuse instance for root path (read-only base layer).
    dicfuse: Arc<Dicfuse>,
    /// Cache of Dicfuse instances keyed by base_path for subdirectory mounts.
    /// This avoids creating duplicate instances for the same path.
    dicfuse_cache: Arc<RwLock<HashMap<String, Arc<Dicfuse>>>>,
    /// Active mounts indexed by UUID.
    mounts: Arc<RwLock<HashMap<Uuid, MountEntry>>>,
    /// Fast lookup for (path, cl) -> mount_id to avoid linear scans.
    path_index: PathIndex,
    /// Fast lookup for (job_id/build_id) -> mount_id for task-granularity mounts.
    job_index: JobIndex,
    /// Service start time for uptime calculation.
    start_time: Instant,
    /// Path to the state file for persistence.
    state_file: PathBuf,
}

impl AntaresServiceImpl {
    /// Create a new service instance.
    ///
    /// # Arguments
    /// * `dicfuse` - Optional shared Dicfuse instance. If None, creates a new one.
    ///
    /// # Note
    /// Requires config to be initialized via `config::init_config()` before calling.
    pub async fn new(dicfuse: Option<Arc<Dicfuse>>) -> Self {
        let dic = match dicfuse {
            Some(d) => d,
            None => DicfuseManager::global().await,
        };
        let state_file = PathBuf::from(crate::util::config::antares_state_file());
        Self {
            dicfuse: dic,
            dicfuse_cache: Arc::new(RwLock::new(HashMap::new())),
            mounts: Arc::new(RwLock::new(HashMap::new())),
            path_index: Arc::new(RwLock::new(HashMap::new())),
            job_index: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
            state_file,
        }
    }

    /// Create a new service instance and recover previous mounts if available.
    ///
    /// # Arguments
    /// * `dicfuse` - Optional shared Dicfuse instance. If None, creates a new one.
    ///
    /// # Note
    /// Requires config to be initialized via `config::init_config()` before calling.
    pub async fn new_with_recovery(dicfuse: Option<Arc<Dicfuse>>) -> Self {
        let instance = Self::new(dicfuse).await;
        instance.recover_mounts().await;
        instance
    }

    /// Get or create a Dicfuse instance for the given path.
    ///
    /// For root path ("/" or empty), returns the shared global instance.
    /// For subdirectory paths, returns a cached instance or creates a new one.
    /// This ensures that multiple mounts with the same base_path share the same
    /// Dicfuse instance, avoiding unnecessary duplication.
    ///
    /// IMPORTANT: For newly created instances, this method waits for the Dicfuse
    /// directory tree to be fully initialized before returning. This prevents
    /// FUSE mount failures due to root inode not being set up yet.
    ///
    /// # TODO(dicfuse-antares-integration)
    /// - Support incremental directory tree loading to reduce initial wait time
    /// - Add progress callback for long-running initialization
    /// - Consider lazy loading for very large subdirectory mounts
    async fn get_or_create_dicfuse(&self, path: &str) -> Result<Arc<Dicfuse>, ServiceError> {
        const INIT_TIMEOUT_SECS: u64 = 120;

        // For root path, use the shared global instance (but ensure it's initialized first).
        if path.is_empty() || path == "/" {
            tracing::info!(
                "Waiting for shared Dicfuse instance to initialize for path: / (timeout: {}s)",
                INIT_TIMEOUT_SECS
            );
            match tokio::time::timeout(
                Duration::from_secs(INIT_TIMEOUT_SECS),
                self.dicfuse.store.wait_for_ready(),
            )
            .await
            {
                Ok(_) => {
                    tracing::info!("Shared Dicfuse initialized successfully for path: /");
                }
                Err(_) => {
                    tracing::error!(
                        "Shared Dicfuse initialization timed out for path: / after {}s",
                        INIT_TIMEOUT_SECS
                    );
                    return Err(ServiceError::FuseFailure(format!(
                        "Dicfuse initialization timed out for path '/' after {}s. \
                         Check network connectivity to the monorepo server.",
                        INIT_TIMEOUT_SECS
                    )));
                }
            }
            return Ok(self.dicfuse.clone());
        }

        // Normalize the path for consistent cache keys
        let normalized_path = path.trim_end_matches('/').to_string();

        // Check cache first - if found, it's already initialized
        {
            let cache = self.dicfuse_cache.read().await;
            if let Some(dicfuse) = cache.get(&normalized_path) {
                tracing::debug!(
                    "Using cached Dicfuse instance for path: {}",
                    normalized_path
                );
                return Ok(dicfuse.clone());
            }
        }

        // Not in cache, create new instance
        let new_dicfuse = DicfuseManager::for_base_path(&normalized_path).await;

        // CRITICAL: Wait for the Dicfuse directory tree to be fully loaded before
        // returning. Without this, FUSE mount may fail because the root inode
        // is not set up yet when import_arc hasn't completed.
        // TODO(dicfuse-antares-integration): If many concurrent requests initialize DIFFERENT
        // base paths, we may enqueue a large number of concurrent warmups (network + memory).
        // Consider adding a global semaphore/queue to cap concurrent initializations.
        tracing::info!(
            "Waiting for Dicfuse instance to initialize for path: {} (timeout: {}s)",
            normalized_path,
            INIT_TIMEOUT_SECS
        );
        match tokio::time::timeout(
            std::time::Duration::from_secs(INIT_TIMEOUT_SECS),
            new_dicfuse.store.wait_for_ready(),
        )
        .await
        {
            Ok(_) => {
                tracing::info!(
                    "Dicfuse initialized successfully for path: {}",
                    normalized_path
                );
            }
            Err(_) => {
                tracing::error!(
                    "Dicfuse initialization timed out for path: {} after {}s",
                    normalized_path,
                    INIT_TIMEOUT_SECS
                );
                return Err(ServiceError::FuseFailure(format!(
                    "Dicfuse initialization timed out for path '{}' after {}s. \
                     Check network connectivity to the monorepo server.",
                    normalized_path, INIT_TIMEOUT_SECS
                )));
            }
        }

        // Insert into cache
        {
            let mut cache = self.dicfuse_cache.write().await;
            // Double-check in case another task created it while we were waiting
            if let Some(dicfuse) = cache.get(&normalized_path) {
                return Ok(dicfuse.clone());
            }
            cache.insert(normalized_path.clone(), new_dicfuse.clone());
            tracing::info!(
                "Created and cached new Dicfuse instance for path: {}",
                normalized_path
            );
        }

        Ok(new_dicfuse)
    }

    /// Persist current mount state to file.
    async fn persist_state(&self) {
        let mounts = self.mounts.read().await;
        let state = PersistedState {
            mounts: mounts
                .values()
                .filter(|e| matches!(e.state, MountLifecycle::Mounted))
                .map(|e| PersistedMountState {
                    mount_id: e.mount_id,
                    job_id: e.job_id.clone(),
                    path: e.path.clone(),
                    cl: e.cl.clone(),
                    mountpoint: e.mountpoint.clone(),
                    upper_dir: e.upper_dir.clone(),
                    cl_dir: e.cl_dir.clone(),
                    created_at_epoch_ms: e.created_at_epoch_ms,
                })
                .collect(),
        };
        drop(mounts);

        // Write state to file
        if let Some(parent) = self.state_file.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::warn!("Failed to create state directory: {}", e);
                return;
            }
        }

        match toml::to_string_pretty(&state) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&self.state_file, content) {
                    tracing::warn!("Failed to write state file: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to serialize state: {}", e);
            }
        }
    }

    /// Recover mounts from persisted state file.
    async fn recover_mounts(&self) {
        if !self.state_file.exists() {
            tracing::debug!(
                "No state file found at {:?}, skipping recovery",
                self.state_file
            );
            return;
        }

        let content = match std::fs::read_to_string(&self.state_file) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read state file: {}", e);
                return;
            }
        };

        let state: PersistedState = match toml::from_str(&content) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to parse state file: {}", e);
                tracing::error!(
                    "Failed to parse state file at {:?}: {}. Skipping mount recovery.",
                    self.state_file,
                    e
                );
                return;
            }
        };

        tracing::info!("Recovering {} mounts from state file", state.mounts.len());

        for persisted in state.mounts {
            // Check if mountpoint still exists
            let mountpoint = PathBuf::from(&persisted.mountpoint);
            if !mountpoint.exists() {
                tracing::info!(
                    "Skipping recovery of mount {} - mountpoint no longer exists",
                    persisted.mount_id
                );
                continue;
            }

            // Get or create Dicfuse instance (uses cache for subdirectory paths)
            let dicfuse = match self.get_or_create_dicfuse(&persisted.path).await {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(
                        "Failed to get Dicfuse for {} during recovery: {}",
                        persisted.mount_id,
                        e
                    );
                    continue;
                }
            };

            let upper_dir = PathBuf::from(&persisted.upper_dir);
            let cl_dir = persisted.cl_dir.as_ref().map(PathBuf::from);

            // Try to create and mount AntaresFuse
            match AntaresFuse::new(mountpoint.clone(), dicfuse, upper_dir, cl_dir.clone()).await {
                Ok(mut fuse) => {
                    if let Err(e) = fuse.mount().await {
                        tracing::warn!(
                            "Failed to remount {} during recovery: {}",
                            persisted.mount_id,
                            e
                        );
                        continue;
                    }

                    // Create entry
                    let entry = MountEntry {
                        mount_id: persisted.mount_id,
                        job_id: persisted.job_id.clone(),
                        path: persisted.path.clone(),
                        cl: persisted.cl.clone(),
                        mountpoint: persisted.mountpoint.clone(),
                        upper_dir: persisted.upper_dir.clone(),
                        cl_dir: persisted.cl_dir.clone(),
                        fuse,
                        state: MountLifecycle::Mounted,
                        created_at_epoch_ms: persisted.created_at_epoch_ms,
                        last_seen_epoch_ms: current_epoch_ms(),
                    };

                    let mut mounts = self.mounts.write().await;
                    let mut index = self.path_index.write().await;
                    let mut job_index = self.job_index.write().await;
                    mounts.insert(persisted.mount_id, entry);
                    if let Some(job_id) = persisted.job_id {
                        job_index.insert(job_id, persisted.mount_id);
                    } else {
                        index.insert((persisted.path, persisted.cl), persisted.mount_id);
                    }

                    tracing::info!("Recovered mount {} at {:?}", persisted.mount_id, mountpoint);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to create AntaresFuse for recovery of {}: {}",
                        persisted.mount_id,
                        e
                    );
                }
            }
        }
    }

    /// Validate the create mount request.
    fn validate_request(request: &CreateMountRequest) -> Result<(), ServiceError> {
        if request.path.is_empty() {
            return Err(ServiceError::InvalidRequest("path cannot be empty".into()));
        }
        Ok(())
    }

    /// Check if a path+cl combination is already mounted.
    async fn is_path_already_mounted(&self, path: &str, cl: Option<&str>) -> bool {
        let index = self.path_index.read().await;
        index.contains_key(&(path.to_string(), cl.map(|s| s.to_string())))
    }

    /// Get service health information.
    pub async fn health_info_impl(&self) -> HealthResponse {
        let mounts = self.mounts.read().await;
        HealthResponse {
            status: "healthy".to_string(),
            mount_count: mounts.len(),
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }

    /// Cleanup all mounts during shutdown.
    pub async fn shutdown_cleanup_impl(&self) -> Result<(), ServiceError> {
        let mut mounts = self.mounts.write().await;
        let mut index = self.path_index.write().await;
        let mut job_index = self.job_index.write().await;

        for (mount_id, mut entry) in mounts.drain() {
            tracing::info!("Unmounting {} during shutdown", mount_id);
            if let Err(e) = entry.fuse.unmount().await {
                tracing::warn!("Failed to unmount {} during shutdown: {}", mount_id, e);
                // Continue with other mounts even if one fails
            }
        }
        // All mounts drained; clear indices.
        // TODO(antares): If we ever decide to keep failed-unmount mounts in memory/state for
        // later retry, revisit index cleanup to avoid inconsistencies.
        index.clear();
        job_index.clear();
        Ok(())
    }
}

#[async_trait]
impl AntaresService for AntaresServiceImpl {
    async fn create_mount(
        &self,
        request: CreateMountRequest,
    ) -> Result<MountCreated, ServiceError> {
        // 1. Validate request
        Self::validate_request(&request)?;

        // Derive a task identifier (job/build id) if provided.
        let task_id: Option<String> = request
            .job_id
            .clone()
            .or(request.build_id.clone())
            .and_then(|s| {
                let trimmed = s.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            });

        // 2. Idempotency / de-dup policy:
        // - If task_id is provided: treat create as idempotent for the same task id.
        //   This supports build-task-granularity mounts.
        // - If task_id is NOT provided: keep legacy behavior and reject duplicate (path, cl).
        if let Some(ref job_id) = task_id {
            // Fast path: already mounted for this task id -> return existing mount.
            if let Some(existing_id) = { self.job_index.read().await.get(job_id).cloned() } {
                let mut mounts = self.mounts.write().await;
                if let Some(entry) = mounts.get_mut(&existing_id) {
                    // Guard against job_id reuse with different request params.
                    if entry.path != request.path || entry.cl != request.cl {
                        return Err(ServiceError::InvalidRequest(format!(
                            "job_id/build_id '{}' already mounted with different path/cl",
                            job_id
                        )));
                    }
                    // If the mount is being torn down, do NOT treat this as an idempotent success.
                    // Otherwise we may return a mount_id that is about to be removed, causing
                    // follow-up describe/delete calls to 404.
                    if !matches!(entry.state, MountLifecycle::Mounted) {
                        return Err(ServiceError::InvalidRequest(format!(
                            "job_id/build_id '{}' is currently in state {:?}; retry after unmount completes",
                            job_id, entry.state
                        )));
                    }
                    entry.update_last_seen();
                    return Ok(MountCreated {
                        mount_id: existing_id,
                        mountpoint: entry.mountpoint.clone(),
                    });
                } else {
                    // Stale index entry: remove and continue with fresh mount creation.
                    self.job_index.write().await.remove(job_id);
                }
            }
        } else if self
            .is_path_already_mounted(&request.path, request.cl.as_deref())
            .await
        {
            return Err(ServiceError::InvalidRequest(format!(
                "path {} with cl {:?} is already mounted",
                request.path, request.cl
            )));
        }

        // 3. Generate UUID and auto-generate all paths
        let mount_id = Uuid::new_v4();
        let id_str = mount_id.to_string();

        // Get base paths from config
        let mount_root = crate::util::config::antares_mount_root();
        let upper_root = crate::util::config::antares_upper_root();
        let cl_root = crate::util::config::antares_cl_root();

        // Auto-generate paths based on UUID
        let mountpoint_str = format!("{}/{}", mount_root, id_str);
        let upper_dir_str = format!("{}/{}", upper_root, id_str);
        let cl_dir_str = request
            .cl
            .as_ref()
            .map(|_| format!("{}/{}", cl_root, id_str));

        let mountpoint = PathBuf::from(&mountpoint_str);
        let upper_dir = PathBuf::from(&upper_dir_str);
        let cl_dir = cl_dir_str.as_ref().map(PathBuf::from);

        // CL layer support removed: skip building CL layer if provided

        // 5. Get or create Dicfuse instance for this mount (uses cache for subdirectory paths)
        // If a specific base path is requested (not root), get from cache or create a dedicated
        // Dicfuse with path remapping. Otherwise, use the shared global instance.
        // This may take time for new subdirectory paths as it waits for import_arc to complete.
        let dicfuse = self.get_or_create_dicfuse(&request.path).await?;

        // 6. Create AntaresFuse instance (may take time, not holding lock)
        let mut fuse = AntaresFuse::new(mountpoint, dicfuse, upper_dir, cl_dir)
            .await
            .map_err(|e| ServiceError::FuseFailure(format!("failed to create fuse: {}", e)))?;

        // 7. Mount the filesystem
        fuse.mount()
            .await
            .map_err(|e| ServiceError::FuseFailure(format!("failed to mount: {}", e)))?;

        // 8. Record timestamps. We'll only construct MountEntry after passing the duplicate check
        // so we can rollback the FUSE mount safely on race losers.
        let now = current_epoch_ms();

        // 9. Insert into mounts map
        let mut mounts = self.mounts.write().await;
        let mut index = self.path_index.write().await;
        let mut job_index = self.job_index.write().await;

        // Double-check for races after acquiring write locks (match the policy above).
        if let Some(ref job_id) = task_id {
            if job_index.contains_key(job_id) {
                // IMPORTANT: rollback the freshly mounted FUSE session before returning.
                // Under concurrent POST /mounts for the same job_id/build_id, the losing request
                // may have already mounted a FUSE session but not yet inserted it into mounts /
                // job_index. Returning early here would leak an orphan mount that cannot be
                // tracked or cleaned up.
                let err = ServiceError::InvalidRequest(format!(
                    "job_id/build_id '{}' is already mounted",
                    job_id
                ));
                drop(mounts);
                drop(index);
                drop(job_index);

                tracing::warn!(
                    "create_mount duplicate task_id detected after mount; rolling back orphan mount {}",
                    mount_id
                );
                let _ = fuse.unmount().await;
                let _ = std::fs::remove_dir_all(&mountpoint_str);
                let _ = std::fs::remove_dir_all(&upper_dir_str);
                if let Some(c) = cl_dir_str.as_deref() {
                    let _ = std::fs::remove_dir_all(c);
                }
                return Err(err);
            }
        } else if index.contains_key(&(request.path.clone(), request.cl.clone())) {
            // Same rollback logic as above for legacy (path, cl) duplicates.
            let err = ServiceError::InvalidRequest(format!(
                "path {} with cl {:?} is already mounted",
                request.path, request.cl
            ));
            drop(mounts);
            drop(index);
            drop(job_index);

            tracing::warn!(
                "create_mount duplicate (path, cl) detected after mount; rolling back orphan mount {}",
                mount_id
            );
            let _ = fuse.unmount().await;
            let _ = std::fs::remove_dir_all(&mountpoint_str);
            let _ = std::fs::remove_dir_all(&upper_dir_str);
            if let Some(c) = cl_dir_str.as_deref() {
                let _ = std::fs::remove_dir_all(c);
            }
            return Err(err);
        }

        // Now it's safe to commit the mount into the in-memory state.
        let entry = MountEntry {
            mount_id,
            job_id: task_id.clone(),
            path: request.path.clone(),
            cl: request.cl.clone(),
            mountpoint: mountpoint_str.clone(),
            upper_dir: upper_dir_str.clone(),
            cl_dir: cl_dir_str.clone(),
            fuse,
            state: MountLifecycle::Mounted,
            created_at_epoch_ms: now,
            last_seen_epoch_ms: now,
        };

        // Preserve path/cl for logging before moving into index
        let path_for_log = request.path.clone();
        let cl_for_log = request.cl.clone();

        mounts.insert(mount_id, entry);
        if let Some(job_id) = task_id {
            job_index.insert(job_id, mount_id);
        } else {
            index.insert((request.path.clone(), request.cl.clone()), mount_id);
        }

        tracing::info!(
            "Created mount {} for path '{}' (cl: {:?}) at {}",
            mount_id,
            path_for_log,
            cl_for_log,
            mountpoint_str
        );

        // IMPORTANT: release locks before persisting state.
        // `persist_state()` acquires `self.mounts.read()`. If we keep holding `mounts.write()`
        // here, the task deadlocks and the HTTP request never returns (curl hangs at step [1]).
        drop(mounts);
        drop(index);
        drop(job_index);

        // Persist state to file for recovery
        self.persist_state().await;

        Ok(MountCreated {
            mount_id,
            mountpoint: mountpoint_str,
        })
    }

    async fn list_mounts(&self) -> Result<Vec<MountStatus>, ServiceError> {
        let mounts = self.mounts.read().await;
        let list: Vec<MountStatus> = mounts.values().map(|e| e.to_status()).collect();
        Ok(list)
    }

    async fn describe_mount(&self, mount_id: Uuid) -> Result<MountStatus, ServiceError> {
        let mounts = self.mounts.read().await;
        let entry = mounts
            .get(&mount_id)
            .ok_or(ServiceError::NotFound(mount_id))?;
        Ok(entry.to_status())
    }

    async fn delete_mount(&self, mount_id: Uuid) -> Result<MountStatus, ServiceError> {
        // Acquire write locks to update state
        let mut mounts = self.mounts.write().await;
        let index = self.path_index.write().await;

        // Get mutable reference to entry (don't remove yet)
        let entry = mounts
            .get_mut(&mount_id)
            .ok_or(ServiceError::NotFound(mount_id))?;

        // Set state to Unmounting while still in the map
        entry.state = MountLifecycle::Unmounting;
        entry.update_last_seen();

        // Store path/cl for index removal, then take ownership of fuse for unmount
        let path = entry.path.clone();
        let cl = entry.cl.clone();
        let job_id = entry.job_id.clone();
        let mountpoint = PathBuf::from(&entry.mountpoint);
        let upper_dir = PathBuf::from(&entry.upper_dir);
        let cl_dir = entry.cl_dir.as_ref().map(PathBuf::from);
        let mut fuse = std::mem::replace(&mut entry.fuse, {
            // Create a placeholder AntaresFuse to replace (will be removed anyway if unmount succeeds)
            // This is safe because we're about to remove the entry on success, or restore fuse on failure
            AntaresFuse::new(
                mountpoint.clone(),
                self.dicfuse.clone(),
                upper_dir.clone(),
                cl_dir.clone(),
            )
            .await
            .map_err(|e| {
                ServiceError::Internal(format!("failed to create placeholder fuse: {}", e))
            })?
        });

        // Release locks before potentially slow unmount operation
        drop(mounts);
        drop(index);

        // Unmount the filesystem
        let unmount_result = fuse.unmount().await;

        // Reacquire locks to update state and remove if needed
        let mut mounts = self.mounts.write().await;
        let mut index = self.path_index.write().await;
        let mut job_index = self.job_index.write().await;

        let entry = match mounts.get_mut(&mount_id) {
            Some(entry) => entry,
            None => {
                tracing::error!(
                    "Mount entry {} missing during unmount; possible race or state bug",
                    mount_id
                );
                drop(mounts);
                drop(index);
                drop(job_index);
                return Err(ServiceError::Internal(format!(
                    "Mount entry {} not found during unmount; this should not happen",
                    mount_id
                )));
            }
        };

        if let Err(e) = unmount_result {
            tracing::error!("Failed to unmount {}: {}", mount_id, e);
            // Put fuse back since unmount failed
            entry.fuse = fuse;
            entry.state = MountLifecycle::Failed {
                reason: format!("unmount failed: {}", e),
            };
            entry.update_last_seen();
            // Do not remove from mounts or index; keep for tracking failed unmounts
            let status = entry.to_status();
            drop(mounts);
            drop(index);
            drop(job_index);
            return Ok(status);
        } else {
            entry.state = MountLifecycle::Unmounted;
            entry.update_last_seen();
            // Remove from mounts and index only after successful unmount
            let status = entry.to_status();
            mounts.remove(&mount_id);
            if let Some(job_id) = job_id {
                job_index.remove(&job_id);
            } else {
                index.remove(&(path, cl));
            }
            drop(mounts);
            drop(index);
            drop(job_index);
            tracing::info!("Deleted mount {}", mount_id);

            // Persist state to file for recovery
            self.persist_state().await;

            Ok(status)
        }
    }

    async fn build_cl(&self, mount_id: Uuid, cl_link: String) -> Result<MountStatus, ServiceError> {
        // Get mount entry and verify it exists
        let mounts = self.mounts.read().await;
        let entry = mounts
            .get(&mount_id)
            .ok_or(ServiceError::NotFound(mount_id))?;
        if !matches!(entry.state, MountLifecycle::Mounted) {
            return Err(ServiceError::InvalidRequest(format!(
                "mount {} is currently in state {:?}; cannot build CL",
                mount_id, entry.state
            )));
        }

        // Get or create CL directory path
        let cl_root = crate::util::config::antares_cl_root();
        let cl_dir_str = format!("{}/{}", cl_root, mount_id);
        let cl_dir_path = PathBuf::from(&cl_dir_str);

        // Store path for build_cl_layer before releasing lock
        let _repo_path = entry.path.clone();

        // Release lock before potentially slow CL layer build
        drop(mounts);

        // CL layer building removed - skipping build_cl_layer for this mount

        // Reacquire lock to update entry
        let mut mounts = self.mounts.write().await;
        let mut index = self.path_index.write().await;

        let entry = mounts.get_mut(&mount_id).ok_or_else(|| {
            // Best-effort cleanup: build_cl created cl_dir_path but the mount vanished.
            let _ = std::fs::remove_dir_all(&cl_dir_path);
            ServiceError::NotFound(mount_id)
        })?;
        if !matches!(entry.state, MountLifecycle::Mounted) {
            // Best-effort cleanup: don't leave behind a CL directory for a mount being torn down.
            let _ = std::fs::remove_dir_all(&cl_dir_path);
            return Err(ServiceError::InvalidRequest(format!(
                "mount {} is currently in state {:?}; cannot build CL",
                mount_id, entry.state
            )));
        }

        // Update entry with new CL info
        let old_cl = entry.cl.clone();
        entry.cl = Some(cl_link.clone());
        entry.cl_dir = Some(cl_dir_str);
        entry.update_last_seen();

        // Update path index if CL changed (legacy mounts only).
        // Task-granularity mounts are keyed by job_id/build_id and are not tracked in path_index.
        if entry.job_id.is_none() && old_cl != entry.cl {
            let path = entry.path.clone();
            // Remove old index entry
            index.remove(&(path.clone(), old_cl));
            // Add new index entry
            index.insert((path, entry.cl.clone()), mount_id);
        }

        let status = entry.to_status();
        tracing::info!(
            "Built CL layer for mount {} with link {}",
            mount_id,
            cl_link
        );
        drop(mounts);
        drop(index);

        // Persist state to file for recovery (CL changes must survive restarts).
        self.persist_state().await;

        Ok(status)
    }

    async fn clear_cl(&self, mount_id: Uuid) -> Result<MountStatus, ServiceError> {
        let mut mounts = self.mounts.write().await;
        let mut index = self.path_index.write().await;

        let entry = mounts
            .get_mut(&mount_id)
            .ok_or(ServiceError::NotFound(mount_id))?;
        if !matches!(entry.state, MountLifecycle::Mounted) {
            return Err(ServiceError::InvalidRequest(format!(
                "mount {} is currently in state {:?}; cannot clear CL",
                mount_id, entry.state
            )));
        }

        if entry.cl.is_none() {
            return Err(ServiceError::InvalidRequest(
                "mount has no CL layer to clear".into(),
            ));
        }

        // Remove CL directory contents
        if let Some(ref cl_dir) = entry.cl_dir {
            let cl_path = PathBuf::from(cl_dir);
            if cl_path.exists() {
                std::fs::remove_dir_all(&cl_path).map_err(|e| {
                    ServiceError::Internal(format!("failed to remove CL directory: {}", e))
                })?;
                // Recreate empty directory
                std::fs::create_dir_all(&cl_path).map_err(|e| {
                    ServiceError::Internal(format!("failed to recreate CL directory: {}", e))
                })?;
            }
        }

        // Update path index (legacy mounts only).
        if entry.job_id.is_none() {
            let path = entry.path.clone();
            let old_cl = entry.cl.clone();
            index.remove(&(path.clone(), old_cl));
            index.insert((path, None), mount_id);
        }

        // Clear CL from entry
        entry.cl = None;
        entry.update_last_seen();

        let status = entry.to_status();
        tracing::info!("Cleared CL layer for mount {}", mount_id);
        drop(mounts);
        drop(index);

        // Persist state to file for recovery (CL changes must survive restarts).
        self.persist_state().await;

        Ok(status)
    }

    async fn health_info(&self) -> HealthResponse {
        self.health_info_impl().await
    }

    async fn shutdown_cleanup(&self) -> Result<(), ServiceError> {
        self.shutdown_cleanup_impl().await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use futures::future::join_all;
    use tower::ServiceExt;

    use super::*;

    /// Mock service for testing HTTP layer without actual FUSE operations
    struct MockAntaresService {
        mounts: Arc<RwLock<HashMap<Uuid, MountStatus>>>,
    }

    impl MockAntaresService {
        fn new() -> Self {
            Self {
                mounts: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl AntaresService for MockAntaresService {
        async fn create_mount(
            &self,
            request: CreateMountRequest,
        ) -> Result<MountCreated, ServiceError> {
            if request.path.is_empty() {
                return Err(ServiceError::InvalidRequest("path cannot be empty".into()));
            }

            let task_id = request.job_id.clone().or(request.build_id.clone());

            // Idempotency / de-dup policy:
            // - If task_id is provided: idempotent per task id.
            // - Otherwise: legacy behavior, reject duplicate (path, cl).
            if let Some(ref job_id) = task_id {
                let mounts = self.mounts.read().await;
                if let Some(existing) = mounts
                    .values()
                    .find(|m| m.job_id.as_deref() == Some(job_id))
                {
                    if existing.path != request.path || existing.cl != request.cl {
                        return Err(ServiceError::InvalidRequest(format!(
                            "job_id/build_id '{}' already mounted with different path/cl",
                            job_id
                        )));
                    }
                    if !matches!(existing.state, MountLifecycle::Mounted) {
                        return Err(ServiceError::InvalidRequest(format!(
                            "job_id/build_id '{}' is currently in state {:?}; retry after unmount completes",
                            job_id, existing.state
                        )));
                    }
                    return Ok(MountCreated {
                        mount_id: existing.mount_id,
                        mountpoint: existing.mountpoint.clone(),
                    });
                }
            } else {
                let mounts = self.mounts.read().await;
                if mounts
                    .values()
                    .any(|m| m.path == request.path && m.cl == request.cl)
                {
                    return Err(ServiceError::InvalidRequest(format!(
                        "path {} with cl {:?} is already mounted",
                        request.path, request.cl
                    )));
                }
            }

            // Auto-generate paths based on UUID
            let mount_id = Uuid::new_v4();
            let id_str = mount_id.to_string();
            let mountpoint = format!("/tmp/mock_mnt/{}", id_str);
            let upper_dir = format!("/tmp/mock_upper/{}", id_str);
            let cl_dir = request
                .cl
                .as_ref()
                .map(|_| format!("/tmp/mock_cl/{}", id_str));

            let status = MountStatus {
                mount_id,
                job_id: task_id.clone(),
                path: request.path,
                cl: request.cl,
                mountpoint: mountpoint.clone(),
                layers: MountLayers {
                    upper: upper_dir,
                    cl: cl_dir,
                    dicfuse: "mock".into(),
                },
                state: MountLifecycle::Mounted,
                created_at_epoch_ms: 0,
                last_seen_epoch_ms: 0,
            };
            self.mounts.write().await.insert(mount_id, status);

            Ok(MountCreated {
                mount_id,
                mountpoint,
            })
        }

        async fn list_mounts(&self) -> Result<Vec<MountStatus>, ServiceError> {
            Ok(self.mounts.read().await.values().cloned().collect())
        }

        async fn describe_mount(&self, mount_id: Uuid) -> Result<MountStatus, ServiceError> {
            self.mounts
                .read()
                .await
                .get(&mount_id)
                .cloned()
                .ok_or(ServiceError::NotFound(mount_id))
        }

        async fn delete_mount(&self, mount_id: Uuid) -> Result<MountStatus, ServiceError> {
            self.mounts
                .write()
                .await
                .remove(&mount_id)
                .map(|mut s| {
                    s.state = MountLifecycle::Unmounted;
                    s
                })
                .ok_or(ServiceError::NotFound(mount_id))
        }

        async fn build_cl(
            &self,
            mount_id: Uuid,
            cl_link: String,
        ) -> Result<MountStatus, ServiceError> {
            let mut mounts = self.mounts.write().await;
            let status = mounts
                .get_mut(&mount_id)
                .ok_or(ServiceError::NotFound(mount_id))?;
            if !matches!(status.state, MountLifecycle::Mounted) {
                return Err(ServiceError::InvalidRequest(format!(
                    "mount {} is currently in state {:?}; cannot build CL",
                    mount_id, status.state
                )));
            }
            status.cl = Some(cl_link);
            status.layers.cl = Some(format!("/tmp/mock_cl/{}", mount_id));
            Ok(status.clone())
        }

        async fn clear_cl(&self, mount_id: Uuid) -> Result<MountStatus, ServiceError> {
            let mut mounts = self.mounts.write().await;
            let status = mounts
                .get_mut(&mount_id)
                .ok_or(ServiceError::NotFound(mount_id))?;
            if !matches!(status.state, MountLifecycle::Mounted) {
                return Err(ServiceError::InvalidRequest(format!(
                    "mount {} is currently in state {:?}; cannot clear CL",
                    mount_id, status.state
                )));
            }
            if status.cl.is_none() {
                return Err(ServiceError::InvalidRequest(
                    "mount has no CL layer to clear".into(),
                ));
            }
            status.cl = None;
            status.layers.cl = None;
            Ok(status.clone())
        }

        async fn health_info(&self) -> HealthResponse {
            let mounts = self.mounts.read().await;
            HealthResponse {
                status: "healthy".to_string(),
                mount_count: mounts.len(),
                uptime_secs: 0,
            }
        }

        async fn shutdown_cleanup(&self) -> Result<(), ServiceError> {
            self.mounts.write().await.clear();
            Ok(())
        }
    }

    fn create_test_router() -> Router {
        let service = Arc::new(MockAntaresService::new());
        let daemon = AntaresDaemon::new(service);
        daemon.router()
    }

    #[tokio::test]
    async fn test_healthcheck() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(health.status, "healthy");
    }

    #[tokio::test]
    async fn test_create_mount_success() {
        let app = create_test_router();

        // Simplified request: only path and optional cl
        let body = serde_json::json!({
            "path": "/third-party/mega"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mounts")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: MountCreated = serde_json::from_slice(&body).unwrap();
        // Mountpoint is auto-generated with UUID
        assert!(created.mountpoint.starts_with("/tmp/mock_mnt/"));
    }

    #[tokio::test]
    async fn test_mount_by_job_and_delete_by_job() {
        let app = create_test_router();

        let body = serde_json::json!({
            "job_id": "job-1",
            "path": "/third-party/mega",
            "cl": "CL123"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mounts")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Describe by job_id
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/mounts/by-job/job-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let status: MountStatus = serde_json::from_slice(&body).unwrap();
        assert_eq!(status.job_id.as_deref(), Some("job-1"));
        assert_eq!(status.path, "/third-party/mega");
        assert_eq!(status.cl.as_deref(), Some("CL123"));

        // Delete by job_id
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/mounts/by-job/job-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let deleted: MountStatus = serde_json::from_slice(&body).unwrap();
        assert_eq!(deleted.job_id.as_deref(), Some("job-1"));
        assert!(matches!(deleted.state, MountLifecycle::Unmounted));

        // Now describe should be 404.
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/mounts/by-job/job-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_mounts_empty() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/mounts")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let collection: MountCollection = serde_json::from_slice(&body).unwrap();
        assert!(collection.mounts.is_empty());
    }

    #[tokio::test]
    async fn test_describe_nonexistent_mount_returns_404() {
        let app = create_test_router();
        let fake_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/mounts/{}", fake_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_error_response_format() {
        let app = create_test_router();
        let fake_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/mounts/{}", fake_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let error: ErrorBody = serde_json::from_slice(&body).unwrap();

        assert_eq!(error.code, "NOT_FOUND");
        assert!(error.error.contains(&fake_id.to_string()));
    }

    #[tokio::test]
    async fn test_empty_path_rejected() {
        let app = create_test_router();

        let body = serde_json::json!({
            "path": ""
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mounts")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let error: ErrorBody = serde_json::from_slice(&body).unwrap();
        assert_eq!(error.code, "INVALID_REQUEST");
    }

    #[tokio::test]
    async fn test_create_mount_with_cl() {
        let app = create_test_router();

        // Request with CL identifier
        let body = serde_json::json!({
            "path": "/third-party/mega",
            "cl": "CL12345"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mounts")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_concurrent_mount_requests() {
        let service = Arc::new(MockAntaresService::new());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let svc = service.clone();
                tokio::spawn(async move {
                    svc.create_mount(CreateMountRequest {
                        job_id: None,
                        build_id: None,
                        path: format!("/project/path{}", i),
                        cl: None,
                    })
                    .await
                })
            })
            .collect();

        for h in handles {
            assert!(h.await.unwrap().is_ok());
        }

        // All 10 mounts should exist
        let mounts = service.list_mounts().await.unwrap();
        assert_eq!(mounts.len(), 10);
    }

    #[tokio::test]
    async fn test_duplicate_path_cl_rejected() {
        let service = Arc::new(MockAntaresService::new());

        let request = CreateMountRequest {
            job_id: None,
            build_id: None,
            path: "/third-party/mega".into(),
            cl: Some("CL123".into()),
        };

        // First mount should succeed
        let result1 = service.create_mount(request.clone()).await;
        assert!(result1.is_ok());

        // Second mount with same path+cl should fail
        let result2 = service.create_mount(request).await;
        assert!(matches!(result2, Err(ServiceError::InvalidRequest(_))));
    }

    #[tokio::test]
    async fn test_job_id_idempotent() {
        let service = Arc::new(MockAntaresService::new());

        let request = CreateMountRequest {
            job_id: Some("job-123".into()),
            build_id: None,
            path: "/third-party/mega".into(),
            cl: Some("CL123".into()),
        };

        let first = service.create_mount(request.clone()).await.unwrap();
        let second = service.create_mount(request).await.unwrap();

        assert_eq!(first.mount_id, second.mount_id);
        assert_eq!(first.mountpoint, second.mountpoint);
    }

    #[tokio::test]
    async fn test_job_id_idempotent_rejected_when_unmounting() {
        let service = Arc::new(MockAntaresService::new());

        let request = CreateMountRequest {
            job_id: Some("job-123".into()),
            build_id: None,
            path: "/third-party/mega".into(),
            cl: Some("CL123".into()),
        };

        let first = service.create_mount(request.clone()).await.unwrap();

        // Simulate a concurrent teardown where job_id is still present but mount is unmounting.
        {
            let mut mounts = service.mounts.write().await;
            let s = mounts.get_mut(&first.mount_id).unwrap();
            s.state = MountLifecycle::Unmounting;
        }

        let second = service.create_mount(request).await;
        assert!(matches!(second, Err(ServiceError::InvalidRequest(_))));
    }

    #[tokio::test]
    async fn test_same_path_cl_different_job_id_allowed() {
        let service = Arc::new(MockAntaresService::new());

        let req1 = CreateMountRequest {
            job_id: Some("job-a".into()),
            build_id: None,
            path: "/third-party/mega".into(),
            cl: Some("CL123".into()),
        };
        let req2 = CreateMountRequest {
            job_id: Some("job-b".into()),
            build_id: None,
            path: "/third-party/mega".into(),
            cl: Some("CL123".into()),
        };

        let r1 = service.create_mount(req1).await;
        let r2 = service.create_mount(req2).await;
        assert!(r1.is_ok());
        assert!(r2.is_ok());

        let mounts = service.list_mounts().await.unwrap();
        assert_eq!(mounts.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_mount_success() {
        let service = Arc::new(MockAntaresService::new());

        // Create a mount
        let created = service
            .create_mount(CreateMountRequest {
                job_id: None,
                build_id: None,
                path: "/third-party/mega".into(),
                cl: None,
            })
            .await
            .unwrap();

        let mount_id = created.mount_id;

        // Delete it
        let deleted = service.delete_mount(mount_id).await.unwrap();
        assert!(matches!(deleted.state, MountLifecycle::Unmounted));

        // Verify it's gone
        let result = service.describe_mount(mount_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_same_path_different_cl_allowed() {
        let service = Arc::new(MockAntaresService::new());

        // Mount with CL1
        let result1 = service
            .create_mount(CreateMountRequest {
                job_id: None,
                build_id: None,
                path: "/third-party/mega".into(),
                cl: Some("CL1".into()),
            })
            .await;
        assert!(result1.is_ok());

        // Mount with CL2 (same path, different CL) should succeed
        let result2 = service
            .create_mount(CreateMountRequest {
                job_id: None,
                build_id: None,
                path: "/third-party/mega".into(),
                cl: Some("CL2".into()),
            })
            .await;
        assert!(result2.is_ok());

        // Should have 2 mounts
        let mounts = service.list_mounts().await.unwrap();
        assert_eq!(mounts.len(), 2);
    }

    /// Test concurrent mount creation to verify thread safety.
    /// This validates that multiple Antares instances can safely share
    /// the same service and create mounts concurrently.
    #[tokio::test]
    async fn test_concurrent_mount_creation() {
        let service = Arc::new(MockAntaresService::new());

        // Spawn 10 concurrent mount creation tasks
        let mut handles = Vec::new();
        for i in 0..10 {
            let svc = service.clone();
            let handle = tokio::spawn(async move {
                let request = CreateMountRequest {
                    job_id: None,
                    build_id: None,
                    path: format!("/concurrent-path-{}", i),
                    cl: None,
                };
                svc.create_mount(request).await
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let results: Vec<_> = join_all(handles).await;

        // All should succeed
        let mut success_count = 0;
        for result in results {
            match result {
                Ok(Ok(_)) => success_count += 1,
                Ok(Err(e)) => panic!("Mount creation failed: {:?}", e),
                Err(e) => panic!("Task panicked: {:?}", e),
            }
        }
        assert_eq!(success_count, 10, "All 10 concurrent mounts should succeed");

        // Verify all mounts are listed
        let mounts = service.list_mounts().await.unwrap();
        assert_eq!(
            mounts.len(),
            10,
            "Should have 10 mounts after concurrent creation"
        );

        // Verify paths are unique
        let paths: std::collections::HashSet<_> = mounts.iter().map(|m| m.path.clone()).collect();
        assert_eq!(paths.len(), 10, "All paths should be unique");
    }

    /// Test concurrent operations on the same mount.
    #[tokio::test]
    async fn test_concurrent_operations_same_mount() {
        let service = Arc::new(MockAntaresService::new());

        // Create a mount
        let request = CreateMountRequest {
            job_id: None,
            build_id: None,
            path: "/test-concurrent-ops".to_string(),
            cl: None,
        };
        let created = service.create_mount(request).await.unwrap();
        let mount_id = created.mount_id;

        // Spawn multiple concurrent describe operations
        let mut handles = Vec::new();
        for _ in 0..20 {
            let svc = service.clone();
            let id = mount_id;
            let handle = tokio::spawn(async move { svc.describe_mount(id).await });
            handles.push(handle);
        }

        // All describe operations should succeed
        let results: Vec<_> = join_all(handles).await;
        for result in results {
            assert!(
                result.is_ok() && result.unwrap().is_ok(),
                "All describe operations should succeed"
            );
        }
    }

    /// Test build_cl API - successfully add CL layer to mount
    #[tokio::test]
    async fn test_build_cl_success() {
        let service = Arc::new(MockAntaresService::new());

        // Create a mount without CL
        let created = service
            .create_mount(CreateMountRequest {
                job_id: None,
                build_id: None,
                path: "/third-party/mega".into(),
                cl: None,
            })
            .await
            .unwrap();

        let mount_id = created.mount_id;

        // Build CL layer
        let status = service.build_cl(mount_id, "CL123".into()).await.unwrap();
        assert_eq!(status.cl, Some("CL123".into()));
        assert!(status.layers.cl.is_some());
    }

    #[tokio::test]
    async fn test_build_cl_rejected_when_unmounting() {
        let service = Arc::new(MockAntaresService::new());

        let created = service
            .create_mount(CreateMountRequest {
                job_id: None,
                build_id: None,
                path: "/third-party/mega".into(),
                cl: None,
            })
            .await
            .unwrap();

        {
            let mut mounts = service.mounts.write().await;
            let s = mounts.get_mut(&created.mount_id).unwrap();
            s.state = MountLifecycle::Unmounting;
        }

        let result = service.build_cl(created.mount_id, "CL123".into()).await;
        assert!(matches!(result, Err(ServiceError::InvalidRequest(_))));
    }

    /// Test build_cl API - mount not found
    #[tokio::test]
    async fn test_build_cl_not_found() {
        let service = Arc::new(MockAntaresService::new());
        let fake_id = Uuid::new_v4();

        let result = service.build_cl(fake_id, "CL123".into()).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    /// Test clear_cl API - successfully clear CL layer
    #[tokio::test]
    async fn test_clear_cl_success() {
        let service = Arc::new(MockAntaresService::new());

        // Create a mount with CL
        let created = service
            .create_mount(CreateMountRequest {
                job_id: None,
                build_id: None,
                path: "/third-party/mega".into(),
                cl: Some("CL123".into()),
            })
            .await
            .unwrap();

        let mount_id = created.mount_id;

        // Clear CL layer
        let status = service.clear_cl(mount_id).await.unwrap();
        assert_eq!(status.cl, None);
        assert!(status.layers.cl.is_none());
    }

    /// Test clear_cl API - no CL layer to clear
    #[tokio::test]
    async fn test_clear_cl_no_layer() {
        let service = Arc::new(MockAntaresService::new());

        // Create a mount without CL
        let created = service
            .create_mount(CreateMountRequest {
                job_id: None,
                build_id: None,
                path: "/third-party/mega".into(),
                cl: None,
            })
            .await
            .unwrap();

        let mount_id = created.mount_id;

        // Try to clear non-existent CL layer
        let result = service.clear_cl(mount_id).await;
        assert!(matches!(result, Err(ServiceError::InvalidRequest(_))));
    }

    /// Test HTTP endpoint for build_cl
    #[tokio::test]
    async fn test_http_build_cl() {
        let service = Arc::new(MockAntaresService::new());

        // First create a mount
        let created = service
            .create_mount(CreateMountRequest {
                job_id: None,
                build_id: None,
                path: "/test/path".into(),
                cl: None,
            })
            .await
            .unwrap();

        let daemon = AntaresDaemon::new(service);
        let app = daemon.router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/mounts/{}/cl", created.mount_id))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"cl":"CL456"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let status: MountStatus = serde_json::from_slice(&body).unwrap();
        assert_eq!(status.cl, Some("CL456".into()));
    }

    /// Test HTTP endpoint for clear_cl
    #[tokio::test]
    async fn test_http_clear_cl() {
        let service = Arc::new(MockAntaresService::new());

        // First create a mount with CL
        let created = service
            .create_mount(CreateMountRequest {
                job_id: None,
                build_id: None,
                path: "/test/path".into(),
                cl: Some("CL123".into()),
            })
            .await
            .unwrap();

        let daemon = AntaresDaemon::new(service);
        let app = daemon.router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/mounts/{}/cl", created.mount_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let status: MountStatus = serde_json::from_slice(&body).unwrap();
        assert_eq!(status.cl, None);
    }
}
