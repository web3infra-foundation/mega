use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{self, Request, StatusCode, Uri};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use clap::Args;
use lazy_static::lazy_static;
use regex::Regex;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use ceres::lfs::LfsConfig;
use ceres::protocol::{ServiceType, SmartProtocol, TransportProtocol};
use common::config::Config;
use common::model::{CommonOptions, GetParams};
use jupiter::context::Context;
use jupiter::raw_storage::local_storage::LocalStorage;

use crate::api::api_router::{self};
use crate::api::oauth::{self, OauthServiceState};
use crate::api::MonoApiServiceState;
use crate::lfs;

#[derive(Args, Clone, Debug)]
pub struct HttpOptions {
    #[clap(flatten)]
    pub common: CommonOptions,

    #[arg(long, default_value_t = 8000)]
    pub http_port: u16,
}

#[derive(Args, Clone, Debug)]
pub struct HttpsOptions {
    #[clap(flatten)]
    pub common: CommonOptions,

    #[arg(long, default_value_t = 443)]
    pub https_port: u16,

    #[arg(long, value_name = "FILE")]
    pub https_key_path: PathBuf,

    #[arg(long, value_name = "FILE")]
    pub https_cert_path: PathBuf,
}

#[derive(Clone)]
pub struct AppState {
    pub context: Context,
    pub host: String,
    pub port: u16,
    pub common: CommonOptions,
}

impl From<AppState> for LfsConfig {
    fn from(value: AppState) -> Self {
        Self {
            host: value.host,
            port: value.port,
            context: value.context.clone(),
            lfs_storage: Arc::new(LocalStorage::init(
                value.context.config.storage.lfs_obj_local_path,
            )),
            repo_name: String::from("repo_name"),
            enable_split: value.context.config.lfs.enable_split,
            split_size: value.context.config.lfs.split_size,
        }
    }
}

pub fn remove_git_suffix(uri: Uri, git_suffix: &str) -> PathBuf {
    PathBuf::from(uri.path().replace(".git", "").replace(git_suffix, ""))
}

pub async fn start_https(config: Config, options: HttpsOptions) {
    let HttpsOptions {
        common: CommonOptions { host, .. },
        https_key_path,
        https_cert_path,
        https_port,
    } = options.clone();

    let app = app(config, host.clone(), https_port, options.common.clone()).await;

    let server_url = format!("{}:{}", host, https_port);
    let addr = SocketAddr::from_str(&server_url).unwrap();
    let config = RustlsConfig::from_pem_file(https_cert_path.to_owned(), https_key_path.to_owned())
        .await
        .unwrap();
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

pub async fn start_http(config: Config, options: HttpOptions) {
    let HttpOptions {
        common: CommonOptions { host, .. },
        http_port,
    } = options.clone();

    let app = app(config, host.clone(), http_port, options.common.clone()).await;

    let server_url = format!("{}:{}", host, http_port);

    let addr = SocketAddr::from_str(&server_url).unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub async fn app(config: Config, host: String, port: u16, common: CommonOptions) -> Router {
    let context = Context::new(config.clone()).await;
    context.services.mono_storage.init_monorepo().await;
    let state = AppState {
        host,
        port,
        context: context.clone(),
        common: common.clone(),
    };

    let api_state = MonoApiServiceState {
        context: context.clone(),
        common: common.clone(),
    };

    // add RequestDecompressionLayer for handle gzip encode
    // add TraceLayer for log record
    // add CorsLayer to add cors header
    Router::new()
        .nest(
            "/api/v1",
            api_router::routers().with_state(api_state.clone()),
        )
        .nest(
            "/auth",
            oauth::routers().with_state(OauthServiceState {
                context,
                sessions: Arc::new(Mutex::new(HashMap::new())),
            }),
        )
        // Using Regular Expressions for Path Matching in Protocol
        .route(
            "/*path",
            get(get_method_router)
                .post(post_method_router)
                .put(put_method_router),
        )
        .layer(
            ServiceBuilder::new().layer(CorsLayer::new().allow_origin(Any).allow_headers(vec![
                http::header::AUTHORIZATION,
                http::header::CONTENT_TYPE,
            ])),
        )
        .layer(TraceLayer::new_for_http())
        .layer(RequestDecompressionLayer::new())
        .with_state(state)
}

lazy_static! {
    /// The [LFS Server Discovery](https://github.com/git-lfs/git-lfs/blob/main/docs/api/server-discovery.md)
    /// document describes the server LFS discovery protocol.
    ///
    /// The following regular expressions are used to match the LFS server discovery protocol.
    ///
    static ref OBJECTS_REGEX: Regex = Regex::new(r"/objects/[a-z0-9]+$").unwrap();
    static ref LOCKS_REGEX: Regex = Regex::new(r"/locks$").unwrap();

    static ref REGEX_LOCKS_VERIFY: Regex = Regex::new(r"/locks/verify$").unwrap();
    static ref REGEX_UNLOCK: Regex = Regex::new(r"/unlock$").unwrap();
    static ref REGEX_OBJECTS_BATCH: Regex = Regex::new(r"/objects/batch$").unwrap();

    static ref REGEX_OBJECTS_CHUNKIDS: Regex = Regex::new(r"/objects/chunkids$").unwrap();

    /// Git Protocol
    static ref INFO_REFS_REGEX: Regex = Regex::new(r"/info/refs$").unwrap();
    static ref REGEX_GIT_UPLOAD_PACK: Regex = Regex::new(r"/git-upload-pack$").unwrap();
    static ref REGEX_GIT_RECEIVE_PACK: Regex = Regex::new(r"/git-receive-pack$").unwrap();
}

pub async fn get_method_router(
    state: State<AppState>,
    Query(params): Query<GetParams>,
    uri: Uri,
) -> Result<Response<Body>, (StatusCode, String)> {
    let lfs_config: LfsConfig = state.deref().to_owned().into();
    // Routing LFS services.
    if OBJECTS_REGEX.is_match(uri.path()) {
        lfs::lfs_download_object(&lfs_config, uri.path()).await
    } else if LOCKS_REGEX.is_match(uri.path()) {
        lfs::lfs_retrieve_lock(&lfs_config, params).await
    } else if INFO_REFS_REGEX.is_match(uri.path()) {
        let pack_protocol = SmartProtocol::new(
            remove_git_suffix(uri, "/info/refs"),
            state.context.clone(),
            TransportProtocol::Http,
        );
        crate::git_protocol::http::git_info_refs(params, pack_protocol).await
    } else {
        Err((
            StatusCode::NOT_FOUND,
            String::from("Operation not supported\n"),
        ))
    }
}

pub async fn post_method_router(
    state: State<AppState>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response, (StatusCode, String)> {
    let lfs_config: LfsConfig = state.deref().to_owned().into();
    // Routing LFS services.
    if REGEX_LOCKS_VERIFY.is_match(uri.path()) {
        lfs::lfs_verify_lock(state, &lfs_config, req).await
    } else if LOCKS_REGEX.is_match(uri.path()) {
        lfs::lfs_create_lock(state, &lfs_config, req).await
    } else if REGEX_UNLOCK.is_match(uri.path()) {
        lfs::lfs_delete_lock(state, &lfs_config, uri.path(), req).await
    } else if REGEX_OBJECTS_BATCH.is_match(uri.path()) {
        lfs::lfs_process_batch(state, &lfs_config, req).await
    } else if REGEX_OBJECTS_CHUNKIDS.is_match(uri.path()) {
        lfs::lfs_fetch_chunk_ids(state, &lfs_config, req).await
    } else if REGEX_GIT_UPLOAD_PACK.is_match(uri.path()) {
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
        Err((
            StatusCode::NOT_FOUND,
            String::from("Operation not supported"),
        ))
    }
}

pub async fn put_method_router(
    state: State<AppState>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let lfs_config: LfsConfig = state.deref().to_owned().into();
    if OBJECTS_REGEX.is_match(uri.path()) {
        lfs::lfs_upload_object(&lfs_config, uri.path(), req).await
    } else {
        Err((
            StatusCode::NOT_FOUND,
            String::from("Operation not supported"),
        ))
    }
}

#[cfg(test)]
mod tests {}
