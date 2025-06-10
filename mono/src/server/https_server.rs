use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Result;
use async_session::MemoryStore;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{self, Request, Uri};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use http::HeaderValue;
use lazy_static::lazy_static;
use regex::Regex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use ceres::protocol::{ServiceType, SmartProtocol, TransportProtocol};
use common::errors::ProtocolError;
use common::model::{CommonHttpOptions, InfoRefsParams};
use jupiter::context::Context;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::api_router::{self};
use crate::api::lfs::lfs_router;
use crate::api::oauth::{self, oauth_client};
use crate::api::MonoApiServiceState;

#[derive(Clone)]
pub struct AppState {
    pub context: Context,
    pub host: String,
    pub port: u16,
}

pub fn remove_git_suffix(uri: Uri, git_suffix: &str) -> PathBuf {
    PathBuf::from(uri.path().replace(".git", "").replace(git_suffix, ""))
}

pub async fn start_http(context: Context, options: CommonHttpOptions) {
    let CommonHttpOptions { host, port } = options.clone();

    let app = app(context, host.clone(), port).await;

    let server_url = format!("{}:{}", host, port);

    let addr = SocketAddr::from_str(&server_url).unwrap();
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
pub async fn app(context: Context, host: String, port: u16) -> Router {
    let state = AppState {
        host: host.clone(),
        port,
        context: context.clone(),
    };

    let config = context.config.clone();
    let api_state = MonoApiServiceState {
        context: context.clone(),
        oauth_client: Some(oauth_client(config.oauth.clone().unwrap()).unwrap()),
        store: Some(MemoryStore::new()),
        listen_addr: format!("http://{}:{}", host, port),
    };

    let cors_origin = HeaderValue::from_str(&config.oauth.clone().unwrap().ui_domain).expect("ui_domain in config not set");
    // add RequestDecompressionLayer for handle gzip encode
    // add TraceLayer for log record
    // add CorsLayer to add cors header
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(lfs_router::routers().with_state(api_state.clone()))
        .nest(
            "/api/v1",
            api_router::routers().with_state(api_state.clone()),
        )
        .nest("/auth", oauth::routers().with_state(api_state.clone()))
        // Using Regular Expressions for Path Matching in Protocol
        .route("/{*path}", get(get_method_router).post(post_method_router))
        .layer(
            ServiceBuilder::new().layer(CorsLayer::new().allow_origin(cors_origin).allow_headers(
                vec![http::header::AUTHORIZATION, http::header::CONTENT_TYPE],
            ).allow_methods(Any)),
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
    state: State<AppState>,
    Query(params): Query<InfoRefsParams>,
    uri: Uri,
) -> Result<Response<Body>, ProtocolError> {
    if INFO_REFS_REGEX.is_match(uri.path()) {
        let pack_protocol = SmartProtocol::new(
            remove_git_suffix(uri, "/info/refs"),
            state.context.clone(),
            TransportProtocol::Http,
        );
        crate::git_protocol::http::git_info_refs(params, pack_protocol).await
    } else {
        Err(ProtocolError::NotFound(
            "Operation not supported".to_owned(),
        ))
    }
}

pub async fn post_method_router(
    state: State<AppState>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response, ProtocolError> {
    if REGEX_GIT_UPLOAD_PACK.is_match(uri.path()) {
        let mut pack_protocol = SmartProtocol::new(
            remove_git_suffix(uri.clone(), "/git-upload-pack"),
            state.context.clone(),
            TransportProtocol::Http,
        );
        pack_protocol.service_type = Some(ServiceType::UploadPack);
        crate::git_protocol::http::git_upload_pack(req, pack_protocol).await
    } else if REGEX_GIT_RECEIVE_PACK.is_match(uri.path()) {
        let mut pack_protocol = SmartProtocol::new(
            remove_git_suffix(uri.clone(), "/git-receive-pack"),
            state.context.clone(),
            TransportProtocol::Http,
        );
        pack_protocol.service_type = Some(ServiceType::ReceivePack);
        crate::git_protocol::http::git_receive_pack(req, pack_protocol).await
    } else {
        return Err(ProtocolError::NotFound(
            "Operation not supported".to_owned(),
        ));
    }
}

pub const GIT_TAG: &str = "git";
pub const MR_TAG: &str = "merge_request";
pub const ISSUE_TAG: &str = "issue";
#[derive(OpenApi)]
#[openapi(
    tags(
        (name = GIT_TAG, description = "Git API endpoints"),
        (name = MR_TAG, description = "Merge Request API endpoints")
    )
)]
struct ApiDoc;

#[cfg(test)]
mod test {
    use std::{fs, io::Write};
    use utoipa::OpenApi;

    use crate::server::https_server::ApiDoc;

    #[test]
    fn generate_swagger_json() {
        let mut file = fs::File::create("gitmono.json").unwrap();
        let json = ApiDoc::openapi().to_pretty_json().unwrap();
        file.write_all(json.as_bytes()).unwrap();
        println!("{}", json);
    }
}
