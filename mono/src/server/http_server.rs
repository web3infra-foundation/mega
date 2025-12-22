use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::extract::FromRef;
use axum::http::{self, Request, Uri};
use axum::response::Response;
use axum::routing::any;
use axum::{Router, ServiceExt, middleware};
use ceres::api_service::cache::GitObjectCache;
use ceres::api_service::state::ProtocolApiState;
use http::{HeaderValue, Method};

use saturn::entitystore::EntityStore;
use time::Duration;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tower::Layer;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};

use ceres::protocol::{ServiceType, SmartProtocol, TransportProtocol};
use common::errors::ProtocolError;
use common::model::{CommonHttpOptions, InfoRefsParams};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::MonoApiServiceState;
use crate::api::api_router::{self};
use crate::api::guard::cedar_guard::cedar_guard;
use crate::api::oauth::campsite_store::CampsiteApiStore;
use crate::api::oauth::oauth_client;
use crate::api::router::lfs_router;
use context::AppContext;

pub fn remove_git_suffix(full_path: &str, git_suffix: &str) -> PathBuf {
    PathBuf::from(full_path.replace(".git", "").replace(git_suffix, ""))
}

/// Spawns a background task to clean up expired Buck upload sessions.
///
/// Returns `None` if cleanup is disabled in configuration.
fn spawn_cleanup_task(ctx: AppContext, token: CancellationToken) -> Option<JoinHandle<()>> {
    let config = ctx.storage.config();
    let buck_config = config.buck.clone().unwrap_or_default();

    if !buck_config.enable_session_cleanup {
        return None;
    }

    let cleanup_storage = ctx.storage.clone();
    let cleanup_interval = buck_config.cleanup_interval;
    let retention_days = buck_config.completed_retention_days;

    Some(tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(cleanup_interval));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        tracing::info!(
            "Buck upload session cleanup task started (interval: {}s, retention: {}d)",
            cleanup_interval,
            retention_days
        );

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match cleanup_storage
                        .buck_storage()
                        .delete_expired_sessions(retention_days)
                        .await
                    {
                        Ok(count) => {
                            if count > 0 {
                                tracing::info!(
                                    "Buck upload cleanup: deleted {} expired sessions",
                                    count
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Buck upload cleanup failed: {}. Will retry in next interval.",
                                e
                            );
                        }
                    }
                }
                _ = token.cancelled() => {
                    tracing::info!("Buck upload cleanup task received shutdown signal");
                    break;
                }
            }
        }

        tracing::info!("Buck upload cleanup task stopped gracefully");
    }))
}

/// Returns a future that completes when the cancellation token is triggered.
async fn shutdown_signal(token: CancellationToken) {
    token.cancelled().await;
}

pub async fn start_http(ctx: AppContext, options: CommonHttpOptions) {
    let CommonHttpOptions { host, port } = options.clone();

    let middleware = tower::util::MapRequestLayer::new(rewrite_lfs_request_uri::<Body>);

    let shutdown_token = CancellationToken::new();
    let cleanup_handle = spawn_cleanup_task(ctx.clone(), shutdown_token.clone());
    let server_token = shutdown_token.clone();

    let app = app(ctx, host.clone(), port).await;
    let app_with_middleware = middleware.layer(app);

    let server_url = format!("{host}:{port}");
    let addr = SocketAddr::from_str(&server_url).unwrap();
    tracing::info!("HTTP server started up!");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let server_future = axum::serve(listener, app_with_middleware.into_make_service())
        .with_graceful_shutdown(shutdown_signal(server_token));

    let server_handle = tokio::spawn(async move {
        if let Err(e) = server_future.await {
            tracing::error!("HTTP server error: {}", e);
        }
    });

    tokio::pin!(server_handle);

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received shutdown signal (Ctrl+C), starting graceful shutdown...");
        }
        result = server_handle.as_mut() => {
            if let Err(e) = result {
                tracing::error!("HTTP server unexpectedly stopped: {}", e);
            }
            tracing::info!("HTTP server stopped, initiating shutdown...");
        }
    }

    tracing::info!("Broadcasting shutdown signal to all tasks...");
    shutdown_token.cancel();

    let (cleanup_result, server_result) = tokio::join!(
        async {
            if let Some(handle) = cleanup_handle {
                match tokio::time::timeout(std::time::Duration::from_secs(30), handle).await {
                    Ok(Ok(_)) => {
                        tracing::info!("Cleanup task stopped successfully");
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Cleanup task panicked: {}", e);
                        Err(())
                    }
                    Err(_) => {
                        // Timeout indicates potential deadlock or extremely slow I/O.
                        tracing::error!(
                            "Cleanup task did not stop within 30s timeout. \
                            This may indicate a deadlock or extremely slow I/O. \
                            The task will be detached and may continue running. \
                            Operators: check DB/Redis connectivity and long-running I/O; \
                            consider increasing cleanup_interval if workloads are heavy."
                        );
                        Err(())
                    }
                }
            } else {
                Ok(())
            }
        },
        async {
            match server_handle.as_mut().await {
                Ok(_) => {
                    tracing::info!("HTTP server stopped gracefully");
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("HTTP server join error: {}", e);
                    Err(())
                }
            }
        }
    );

    match (cleanup_result, server_result) {
        (Ok(_), Ok(_)) => {
            tracing::info!("Graceful shutdown completed successfully");
        }
        _ => {
            tracing::warn!("Graceful shutdown completed with some errors");
        }
    }
}

/// This is the main entry for the mono server.
/// It is responsible for creating the main router and setting up the necessary middleware.
///
/// The main router is composed of three nested routers:
/// 1. The LFS router nested in the `/`:
///   - GET or PUT `/objects/:object_id`
///   - GET or PUT `/locks`
///   - POST       `/locks/verify`
///   - POST       `/locks/:id/unlock`
///   - GET        `/objects/:object_id/chunks/:chunk_id`
///   - POST       `/objects/batch`
/// 2. The API router nested in the `/api/v1`:
///   - GET        `/api/v1/status`
///   - POST       `/api/v1/create-file`
///   - GET        `/api/v1/latest-commit`
///   - GET        `/api/v1/tree/commit-info`
///   - GET        `/api/v1/tree`
///   - GET        `/api/v1/blob`
///   - GET        `/api/v1/file/blob/:object_id`
///   - GET        `/api/v1/file/tree`
///   - GET        `/api/v1/path-can-clone`
/// 3. The OAuth router nested in the `/auth`:
///   - GET        `/auth/github`
///   - GET        `/auth/authorized`
///   - GET        `/auth/logout`
/// 4. The other routers for the git protocol:
///   - GET        end of `Regex::new(r"/info/refs$")`
///   - POST       end of `Regex::new(r"/git-upload-pack$")`
///   - POST       end of `Regex::new(r"/git-receive-pack$")`
pub async fn app(ctx: AppContext, host: String, port: u16) -> Router {
    let storage = ctx.storage;
    let config = storage.config();

    let oauth_config = config.oauth.clone().unwrap_or_default();
    let git_object_cache = Arc::new(GitObjectCache {
        connection: ctx.connection.clone(),
        prefix: "git-object-bincode".to_string(),
    });

    let api_state = MonoApiServiceState {
        storage: storage.clone(),
        oauth_client: Some(oauth_client(oauth_config.clone()).unwrap()),
        session_store: Some(CampsiteApiStore::new(
            oauth_config.campsite_api_domain,
            storage.user_storage(),
        )),
        listen_addr: format!("http://{host}:{port}"),
        entity_store: EntityStore::new(),
        git_object_cache,
    };

    let origins: Vec<HeaderValue> = oauth_config
        .allowed_cors_origins
        .into_iter()
        .map(|x| x.trim().parse::<HeaderValue>().unwrap())
        .collect();

    // add RequestDecompressionLayer for handle gzip encode
    // add TraceLayer for log record
    // add CorsLayer to add cors header
    // add SessionManagerLayer for session management
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // Set to true in production with HTTPS
        .with_expiry(Expiry::OnInactivity(Duration::seconds(3600))); // 1 hour of inactivity

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(lfs_router::routers().with_state(api_state.clone()))
        .nest(
            "/api/v1",
            api_router::routers()
                .with_state(api_state.clone())
                .route_layer(middleware::from_fn_with_state(
                    api_state.clone(),
                    cedar_guard,
                )),
        )
        // .nest("/auth", oauth::routers().with_state(api_state.clone()))
        // Using Regular Expressions for Path Matching in Protocol
        .route(
            "/{*path}",
            any({
                let api_state = api_state.clone();
                move |req: Request<Body>| {
                    handle_smart_protocol(req, Arc::new(ProtocolApiState::from_ref(&api_state)))
                }
            }),
        )
        .layer(
            ServiceBuilder::new().layer(session_layer).layer(
                CorsLayer::new()
                    .allow_origin(origins)
                    .allow_headers(vec![
                        http::header::AUTHORIZATION,
                        http::header::CONTENT_TYPE,
                    ])
                    .allow_methods([
                        Method::GET,
                        Method::POST,
                        Method::OPTIONS,
                        Method::DELETE,
                        Method::PUT,
                    ])
                    .allow_credentials(true),
            ),
        )
        .layer(TraceLayer::new_for_http())
        .layer(RequestDecompressionLayer::new())
        .with_state(api_state.clone())
        .split_for_parts();

    // Register /info/lfs paths for runtime compatibility (not in OpenAPI)
    // Convert OpenApiRouter to Router to avoid including /info/lfs in OpenAPI docs
    let info_lfs_router: Router = lfs_router::lfs_routes()
        .with_state(api_state.clone())
        .into();

    router
        .nest("/info/lfs", info_lfs_router)
        .merge(SwaggerUi::new("/swagger-ui").url("/api/openapi.json", api))
}

fn rewrite_lfs_request_uri<B>(mut req: Request<B>) -> Request<B> {
    let full_path = req.uri().path();

    if let Some(pos) = full_path.rfind("/info/lfs/") {
        let lfs_subpath = &full_path[pos..];

        let new_path_and_query = if let Some(query) = req.uri().query() {
            format!("{}?{}", lfs_subpath, query)
        } else {
            lfs_subpath.to_owned()
        };

        let new_uri = match Uri::builder().path_and_query(&new_path_and_query).build() {
            Ok(uri) => uri,
            Err(e) => {
                tracing::warn!(
                    "Failed to rewrite LFS URI: {}, error: {}",
                    new_path_and_query,
                    e
                );
                // Return the request unchanged, let downstream handlers deal with it
                return req;
            }
        };

        tracing::debug!("rewrite: old uri {:?}", req.uri());
        *req.uri_mut() = new_uri;
        tracing::debug!("rewrite: new uri {:?}", req.uri());
    }
    req
}

async fn handle_smart_protocol(
    req: Request<Body>,
    state: Arc<ProtocolApiState>,
) -> Result<Response, ProtocolError> {
    let full_path = req.uri().path();
    if full_path.ends_with("/info/refs") && req.method().eq(&Method::GET) {
        let pack_protocol = SmartProtocol::new(
            remove_git_suffix(full_path, "/info/refs"),
            TransportProtocol::Http,
        );
        let uri = req.uri();
        let query_str = uri.query().unwrap_or("");
        let params: InfoRefsParams = serde_urlencoded::from_str(query_str).unwrap();
        crate::git_protocol::http::git_info_refs(&state, params, pack_protocol).await
    } else if full_path.ends_with("/git-upload-pack") && req.method().eq(&Method::POST) {
        let mut pack_protocol = SmartProtocol::new(
            remove_git_suffix(full_path, "/git-upload-pack"),
            TransportProtocol::Http,
        );
        pack_protocol.service_type = Some(ServiceType::UploadPack);
        crate::git_protocol::http::git_upload_pack(&state, req, pack_protocol).await
    } else if full_path.ends_with("/git-receive-pack") && req.method().eq(&Method::POST) {
        let mut pack_protocol = SmartProtocol::new(
            remove_git_suffix(full_path, "/git-receive-pack"),
            TransportProtocol::Http,
        );
        pack_protocol.service_type = Some(ServiceType::ReceivePack);
        crate::git_protocol::http::git_receive_pack(&state, req, pack_protocol).await
    } else {
        Ok(Response::builder()
            .status(404)
            .body(Body::from("Operation not supported"))
            .unwrap())
    }
}

/// Swagger API tag
pub const SYSTEM_COMMON: &str = "System Common";
pub const CODE_PREVIEW: &str = "Code Preview";
pub const TAG_MANAGE: &str = "Tag Management";
pub const CL_TAG: &str = "Change List";
pub const GPG_TAG: &str = "Gpg Key";
pub const ISSUE_TAG: &str = "Issue Management";
pub const SIDEBAR_TAG: &str = "Sidebar Management";
pub const LABEL_TAG: &str = "Label Management";
pub const CONV_TAG: &str = "Conversation and Comment";
pub const SYNC_NOTES_STATE_TAG: &str = "sync-notes-state";
pub const USER_TAG: &str = "User Management";
pub const REPO_TAG: &str = "Repo creation and synchronisation";
pub const MERGE_QUEUE_TAG: &str = "Merge Queue Management";
pub const BUCK_TAG: &str = "Buck Upload API";
pub const LFS_TAG: &str = "Git LFS";
#[derive(OpenApi)]
#[openapi()]
struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;
    use http::Request;

    #[test]
    fn test_rewrite_lfs_uri_basic() {
        let req = Request::builder()
            .uri("/repo/a/b/info/lfs/objects/123")
            .body(())
            .unwrap();

        let new_req = rewrite_lfs_request_uri(req);

        assert_eq!(new_req.uri().path(), "/info/lfs/objects/123");
    }

    #[test]
    fn test_rewrite_keeps_query_string() {
        let req = Request::builder()
            .uri("/repo/a/info/lfs/locks?token=abc123")
            .body(())
            .unwrap();

        let new_req = rewrite_lfs_request_uri(req);

        assert_eq!(
            new_req.uri().path_and_query().unwrap().to_string(),
            "/info/lfs/locks?token=abc123"
        );
    }

    #[test]
    fn test_no_rewrite_when_no_lfs_prefix() {
        let req = Request::builder().uri("/not-lfs-path").body(()).unwrap();

        let new_req = rewrite_lfs_request_uri(req);

        assert_eq!(new_req.uri().path(), "/not-lfs-path");
    }

    #[test]
    fn test_rewrite_with_trailing_slash() {
        let req = Request::builder()
            .uri("/repo/info/lfs/locks/")
            .body(())
            .unwrap();

        let new_req = rewrite_lfs_request_uri(req);

        assert_eq!(new_req.uri().path(), "/info/lfs/locks/");
    }

    #[test]
    fn test_rewrite_complex_path() {
        let req = Request::builder()
            .uri("/a/b/c/info/lfs/objects/abc/def/ghi")
            .body(())
            .unwrap();

        let new_req = rewrite_lfs_request_uri(req);

        assert_eq!(new_req.uri().path(), "/info/lfs/objects/abc/def/ghi");
    }

    #[test]
    fn test_rewrite_when_repo_path_contains_info_lfs() {
        let req = Request::builder()
            .uri("/repos/info/lfs/info/lfs/objects/123")
            .body(())
            .unwrap();

        let new_req = rewrite_lfs_request_uri(req);

        assert_eq!(new_req.uri().path(), "/info/lfs/objects/123");
    }
}
