//! Antares daemon HTTP interface for mount lifecycle management.
//!
//! Provides Axum routes to create, list, query, and delete FUSE mounts backed by
//! AntaresService implementations. Includes graceful shutdown with cleanup.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::{net::SocketAddr, sync::Arc, time::Duration};

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
use tokio::sync::RwLock;
use tokio::time::timeout;
use uuid::Uuid;

use crate::antares::fuse::AntaresFuse;
use crate::dicfuse::Dicfuse;

/// High-level HTTP daemon that exposes Antares orchestration capabilities.
pub struct AntaresDaemon<S: AntaresService> {
    bind_addr: SocketAddr,
    service: Arc<S>,
    shutdown_timeout: Duration,
}

impl<S> AntaresDaemon<S>
where
    S: AntaresService + 'static,
{
    /// Construct a daemon bound to the provided socket and backed by the given service.
    pub fn new(bind_addr: SocketAddr, service: Arc<S>) -> Self {
        Self {
            bind_addr,
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
            .route("/mounts/{mount_id}", get(Self::describe_mount))
            .route("/mounts/{mount_id}", delete(Self::delete_mount))
            .with_state(self.service.clone())
    }

    /// Run the HTTP server until it receives a shutdown signal.
    /// Note: For graceful shutdown with mount cleanup, use AntaresDaemon<AntaresServiceImpl>.
    pub async fn serve(self) -> Result<(), ApiError> {
        let router = self.router();
        let shutdown_timeout = self.shutdown_timeout;
        let service = self.service.clone();

        let listener = tokio::net::TcpListener::bind(self.bind_addr)
            .await
            .map_err(|e| {
                ApiError::Service(ServiceError::Internal(format!(
                    "failed to bind to {}: {}",
                    self.bind_addr, e
                )))
            })?;

        tracing::info!("Antares daemon listening on {}", self.bind_addr);

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
    /// Monorepo path to mount (e.g., "/third-party/mega")
    pub path: String,
    /// Optional CL (changelist) identifier for the CL layer
    #[serde(default)]
    pub cl: Option<String>,
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

/// Concrete implementation of AntaresService.
pub struct AntaresServiceImpl {
    /// Shared Dicfuse instance (read-only base layer).
    dicfuse: Arc<Dicfuse>,
    /// Active mounts indexed by UUID.
    mounts: Arc<RwLock<HashMap<Uuid, MountEntry>>>,
    /// Fast lookup for (path, cl) -> mount_id to avoid linear scans.
    path_index: PathIndex,
    /// Service start time for uptime calculation.
    start_time: Instant,
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
            None => Arc::new(Dicfuse::new().await),
        };
        Self {
            dicfuse: dic,
            mounts: Arc::new(RwLock::new(HashMap::new())),
            path_index: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
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

        for (mount_id, mut entry) in mounts.drain() {
            tracing::info!("Unmounting {} during shutdown", mount_id);
            if let Err(e) = entry.fuse.unmount().await {
                tracing::warn!("Failed to unmount {} during shutdown: {}", mount_id, e);
                // Continue with other mounts even if one fails
            }
            index.retain(|_, v| v != &mount_id);
        }
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

        // 2. Check if path+cl combination is already mounted
        if self
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

        // 4. Create AntaresFuse instance (may take time, not holding lock)
        let mut fuse = AntaresFuse::new(mountpoint, self.dicfuse.clone(), upper_dir, cl_dir)
            .await
            .map_err(|e| ServiceError::FuseFailure(format!("failed to create fuse: {}", e)))?;

        // 5. Mount the filesystem
        fuse.mount()
            .await
            .map_err(|e| ServiceError::FuseFailure(format!("failed to mount: {}", e)))?;

        // 6. Create entry
        let now = current_epoch_ms();

        let entry = MountEntry {
            mount_id,
            path: request.path.clone(),
            cl: request.cl.clone(),
            mountpoint: mountpoint_str.clone(),
            upper_dir: upper_dir_str,
            cl_dir: cl_dir_str,
            fuse,
            state: MountLifecycle::Mounted,
            created_at_epoch_ms: now,
            last_seen_epoch_ms: now,
        };

        // 7. Insert into mounts map
        let mut mounts = self.mounts.write().await;
        let mut index = self.path_index.write().await;

        // Double-check for races after acquiring write locks
        if index.contains_key(&(request.path.clone(), request.cl.clone())) {
            return Err(ServiceError::InvalidRequest(format!(
                "path {} with cl {:?} is already mounted",
                request.path, request.cl
            )));
        }

        // Preserve path/cl for logging before moving into index
        let path_for_log = request.path.clone();
        let cl_for_log = request.cl.clone();

        mounts.insert(mount_id, entry);
        index.insert((request.path, request.cl), mount_id);

        tracing::info!(
            "Created mount {} for path '{}' (cl: {:?}) at {}",
            mount_id,
            path_for_log,
            cl_for_log,
            mountpoint_str
        );

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

        let entry = mounts
            .get_mut(&mount_id)
            .expect("Mount entry must exist during unmount");

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
            return Ok(status);
        } else {
            entry.state = MountLifecycle::Unmounted;
            entry.update_last_seen();
            // Remove from mounts and index only after successful unmount
            let status = entry.to_status();
            mounts.remove(&mount_id);
            index.remove(&(path, cl));
            drop(mounts);
            drop(index);
            tracing::info!("Deleted mount {}", mount_id);
            Ok(status)
        }
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
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

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

            // Check for duplicate path+cl
            {
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
        let daemon = AntaresDaemon::new("127.0.0.1:0".parse().unwrap(), service);
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
    async fn test_delete_mount_success() {
        let service = Arc::new(MockAntaresService::new());

        // Create a mount
        let created = service
            .create_mount(CreateMountRequest {
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
                path: "/third-party/mega".into(),
                cl: Some("CL1".into()),
            })
            .await;
        assert!(result1.is_ok());

        // Mount with CL2 (same path, different CL) should succeed
        let result2 = service
            .create_mount(CreateMountRequest {
                path: "/third-party/mega".into(),
                cl: Some("CL2".into()),
            })
            .await;
        assert!(result2.is_ok());

        // Should have 2 mounts
        let mounts = service.list_mounts().await.unwrap();
        assert_eq!(mounts.len(), 2);
    }
}
