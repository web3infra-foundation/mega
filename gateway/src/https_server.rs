use std::net::SocketAddr;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{Request, StatusCode, Uri};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use clap::Args;
use regex::Regex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use ceres::lfs::LfsConfig;
use ceres::protocol::{SmartProtocol, TransportProtocol};
use common::config::Config;
use common::model::{CommonOptions, GetParams};
use jupiter::context::Context;
use jupiter::raw_storage::local_storage::LocalStorage;

use crate::api_service::router::ApiServiceState;
use crate::{api_service, lfs};

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
        }
    }
}

pub fn remove_git_suffix(uri: Uri, git_suffix: &str) -> PathBuf {
    PathBuf::from(uri.path().replace(".git", "").replace(git_suffix, ""))
}

pub async fn https_server(config: Config, options: HttpsOptions) {
    let HttpsOptions {
        common: CommonOptions { host },
        https_key_path,
        https_cert_path,
        https_port,
    } = options;

    let app = app(config, host.clone(), https_port).await;

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

pub async fn http_server(config: Config, options: HttpOptions) {
    let HttpOptions {
        common: CommonOptions { host },
        http_port,
    } = options;

    let app = app(config, host.clone(), http_port).await;

    let server_url = format!("{}:{}", host, http_port);

    let addr = SocketAddr::from_str(&server_url).unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub async fn app(config: Config, host: String, port: u16) -> Router {
    let context = Context::new(config.clone()).await;
    let state = AppState {
        host,
        port,
        context: context.clone(),
    };

    let api_state = ApiServiceState {
        context,
    };

    // add RequestDecompressionLayer for handle gzip encode
    // add TraceLayer for log record
    // add CorsLayer to add cors header
    Router::new()
        .nest(
            "/api/v1",
            api_service::router::routers().with_state(api_state),
        )
        .route(
            "/*path",
            get(get_method_router)
                .post(post_method_router)
                .put(put_method_router),
        )
        .layer(ServiceBuilder::new().layer(CorsLayer::new().allow_origin(Any)))
        .layer(TraceLayer::new_for_http())
        .layer(RequestDecompressionLayer::new())
        .with_state(state)
}

async fn get_method_router(
    state: State<AppState>,
    Query(params): Query<GetParams>,
    uri: Uri,
) -> Result<Response<Body>, (StatusCode, String)> {
    let lfs_config: LfsConfig = state.deref().to_owned().into();
    // Routing LFS services.
    if Regex::new(r"/objects/[a-z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        lfs::lfs_download_object(&lfs_config, uri.path()).await
    } else if Regex::new(r"/locks$").unwrap().is_match(uri.path()) {
        return lfs::lfs_retrieve_lock(&lfs_config, params).await;
    } else if Regex::new(r"/info/refs$").unwrap().is_match(uri.path()) {
        let pack_protocol = SmartProtocol::new(
            remove_git_suffix(uri, "/info/refs"),
            state.context.clone(),
            TransportProtocol::Http,
        );
        return ceres::http::handler::git_info_refs(params, pack_protocol).await;
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            String::from("Operation not supported\n"),
        ));
    }
}

async fn post_method_router(
    state: State<AppState>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response, (StatusCode, String)> {
    let lfs_config: LfsConfig = state.deref().to_owned().into();
    // Routing LFS services.
    if Regex::new(r"/locks/verify$").unwrap().is_match(uri.path()) {
        lfs::lfs_verify_lock(state, &lfs_config, req).await
    } else if Regex::new(r"/locks$").unwrap().is_match(uri.path()) {
        return lfs::lfs_create_lock(state, &lfs_config, req).await;
    } else if Regex::new(r"/unlock$").unwrap().is_match(uri.path()) {
        return lfs::lfs_delete_lock(state, &lfs_config, uri.path(), req).await;
    } else if Regex::new(r"/objects/batch$").unwrap().is_match(uri.path()) {
        return lfs::lfs_process_batch(state, &lfs_config, req).await;
    } else if Regex::new(r"/git-upload-pack$")
        .unwrap()
        .is_match(uri.path())
    {
        let pack_protocol = SmartProtocol::new(
            remove_git_suffix(uri, "/git-upload-pack"),
            state.context.clone(),
            TransportProtocol::Http,
        );
        ceres::http::handler::git_upload_pack(req, pack_protocol).await
    } else if Regex::new(r"/git-receive-pack$")
        .unwrap()
        .is_match(uri.path())
    {
        let pack_protocol = SmartProtocol::new(
            remove_git_suffix(uri, "/git-receive-pack"),
            state.context.clone(),
            TransportProtocol::Http,
        );
        ceres::http::handler::git_receive_pack(req, pack_protocol).await
    } else {
        Err((
            StatusCode::NOT_FOUND,
            String::from("Operation not supported"),
        ))
    }
}

async fn put_method_router(
    state: State<AppState>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let lfs_config: LfsConfig = state.deref().to_owned().into();
    if Regex::new(r"/objects/[a-z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
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
