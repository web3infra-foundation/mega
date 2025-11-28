use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{self, Request, Uri};
use axum::response::Response;
use axum::routing::get;
use axum::{Router, middleware};
use ceres::api_service::cache::GitObjectCache;
use ceres::api_service::state::ProtocolApiState;
use http::{HeaderValue, Method};
use lazy_static::lazy_static;
use regex::Regex;
use saturn::entitystore::EntityStore;
use time::Duration;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};

use ceres::model::blame::{BlameBlock, BlameInfo, BlameQuery, BlameRequest, BlameResult};
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

pub fn remove_git_suffix(uri: Uri, git_suffix: &str) -> PathBuf {
    PathBuf::from(uri.path().replace(".git", "").replace(git_suffix, ""))
}

pub async fn start_http(ctx: AppContext, options: CommonHttpOptions) {
    let CommonHttpOptions { host, port } = options.clone();

    let app = app(ctx, host.clone(), port).await;

    let server_url = format!("{host}:{port}");

    let addr = SocketAddr::from_str(&server_url).unwrap();
    tracing::info!("HTTP server started up!");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
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
    let state = ProtocolApiState {
        storage: storage.clone(),
        git_object_cache: git_object_cache.clone(),
    };

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
                .route_layer(middleware::from_fn_with_state(api_state, cedar_guard)),
        )
        // .nest("/auth", oauth::routers().with_state(api_state.clone()))
        // Using Regular Expressions for Path Matching in Protocol
        .route("/{*path}", get(get_method_router).post(post_method_router))
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
        .with_state(state)
        .split_for_parts();
    router.merge(SwaggerUi::new("/swagger-ui").url("/api/openapi.json", api))
}

lazy_static! {
    /// The following regular expressions are used to match the Git server protocol.
    static ref INFO_REFS_REGEX: Regex = Regex::new(r"/info/refs$").unwrap();
    static ref REGEX_GIT_UPLOAD_PACK: Regex = Regex::new(r"/git-upload-pack$").unwrap();
    static ref REGEX_GIT_RECEIVE_PACK: Regex = Regex::new(r"/git-receive-pack$").unwrap();
}

pub async fn get_method_router(
    state: State<ProtocolApiState>,
    Query(params): Query<InfoRefsParams>,
    uri: Uri,
) -> Result<Response<Body>, ProtocolError> {
    if INFO_REFS_REGEX.is_match(uri.path()) {
        let pack_protocol = SmartProtocol::new(
            remove_git_suffix(uri, "/info/refs"),
            TransportProtocol::Http,
        );
        crate::git_protocol::http::git_info_refs(&state, params, pack_protocol).await
    } else {
        Err(ProtocolError::NotFound(
            "Operation not supported".to_owned(),
        ))
    }
}

pub async fn post_method_router(
    state: State<ProtocolApiState>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response, ProtocolError> {
    if REGEX_GIT_UPLOAD_PACK.is_match(uri.path()) {
        let mut pack_protocol = SmartProtocol::new(
            remove_git_suffix(uri.clone(), "/git-upload-pack"),
            TransportProtocol::Http,
        );
        pack_protocol.service_type = Some(ServiceType::UploadPack);
        crate::git_protocol::http::git_upload_pack(&state, req, pack_protocol).await
    } else if REGEX_GIT_RECEIVE_PACK.is_match(uri.path()) {
        let mut pack_protocol = SmartProtocol::new(
            remove_git_suffix(uri.clone(), "/git-receive-pack"),
            TransportProtocol::Http,
        );
        pack_protocol.service_type = Some(ServiceType::ReceivePack);
        crate::git_protocol::http::git_receive_pack(&state, req, pack_protocol).await
    } else {
        Err(ProtocolError::NotFound(
            "Operation not supported".to_owned(),
        ))
    }
}

/// Swagger API tag
pub const SYSTEM_COMMON: &str = "System Common";
pub const CODE_PREVIEW: &str = "Code Preview";
pub const TAG_MANAGE: &str = "Tag Management";
pub const CL_TAG: &str = "Change List";
pub const GPG_TAG: &str = "Gpg Key";
pub const ISSUE_TAG: &str = "Issue Management";
pub const LABEL_TAG: &str = "Label Management";
pub const CONV_TAG: &str = "Conversation and Comment";
pub const SYNC_NOTES_STATE_TAG: &str = "sync-notes-state";
pub const USER_TAG: &str = "User Management";
pub const REPO_TAG: &str = "Repo creation and synchronisation";
pub const MERGE_QUEUE_TAG: &str = "Merge Queue Management";
#[derive(OpenApi)]
#[openapi(
    tags(
        (name = CODE_PREVIEW, description = "Git API endpoints"),
        (name = CL_TAG, description = "Change List API endpoints"),
        (name = MERGE_QUEUE_TAG, description = "Merge Queue Management API endpoints")
    ),
    components(schemas(
        BlameBlock,
        BlameInfo,
        BlameQuery,
        BlameRequest,
        BlameResult,
    ))
)]
struct ApiDoc;

#[cfg(test)]
mod test {}
