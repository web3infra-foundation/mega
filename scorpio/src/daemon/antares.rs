//! Suggested Antares daemon HTTP interface.
//!
//! This module intentionally focuses on the REST surface area and traits that the
//! runtime should implement. All functions are left as `todo!()` placeholders so
//! future changes can fill in the actual orchestration logic without rewriting
//! the API shape.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

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
        todo!("Expose Axum router once handlers are implemented");
    }

    /// Run the HTTP server until it receives a shutdown signal.
    pub async fn serve(self) -> Result<(), ApiError> {
        todo!("Bind hyper server and serve router");
    }

    /// Lightweight health/liveness probe.
    async fn healthcheck() -> impl IntoResponse {
        todo!("Return health payload");
    }

    async fn create_mount(
        State(service): State<Arc<S>>,
        Json(request): Json<CreateMountRequest>,
    ) -> Result<Json<MountCreated>, ApiError> {
        todo!("Delegate to AntaresService::create_mount");
    }

    async fn list_mounts(State(service): State<Arc<S>>) -> Result<Json<MountCollection>, ApiError> {
        todo!("Delegate to AntaresService::list_mounts");
    }

    async fn describe_mount(
        State(service): State<Arc<S>>,
        Path(mount_id): Path<Uuid>,
    ) -> Result<Json<MountStatus>, ApiError> {
        todo!("Delegate to AntaresService::describe_mount");
    }

    async fn delete_mount(
        State(service): State<Arc<S>>,
        Path(mount_id): Path<Uuid>,
    ) -> Result<Json<MountStatus>, ApiError> {
        todo!("Delegate to AntaresService::delete_mount");
    }
}

/// Asynchronous service boundary that the HTTP layer depends on.
#[async_trait]
pub trait AntaresService: Send + Sync {
    async fn create_mount(&self, request: CreateMountRequest) -> Result<MountStatus, ServiceError>;
    async fn list_mounts(&self) -> Result<Vec<MountStatus>, ServiceError>;
    async fn describe_mount(&self, mount_id: Uuid) -> Result<MountStatus, ServiceError>;
    async fn delete_mount(&self, mount_id: Uuid) -> Result<MountStatus, ServiceError>;
}

/// Request payload for provisioning a new mount.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateMountRequest {
    /// Absolute path where the mount should appear on the host.
    pub mountpoint: String,
    /// Upper (read-write) directory unique per mount.
    pub upper_dir: String,
    /// Optional CL passthrough directory.
    pub cl_dir: Option<String>,
    /// Arbitrary labels that the orchestrator can apply to the mount.
    pub labels: Vec<String>,
    /// Whether the presented filesystem should be mounted read-only.
    pub readonly: bool,
}

/// Summary returned immediately after provisioning succeeds.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MountCreated {
    pub mount_id: Uuid,
    pub mountpoint: String,
    pub state: MountLifecycle,
}

/// Snapshot of a single mount's state.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MountStatus {
    pub mount_id: Uuid,
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
        todo!("Translate ApiError into Axum response");
    }
}
