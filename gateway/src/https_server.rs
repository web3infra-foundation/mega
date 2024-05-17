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

    #[clap(flatten)]
    pub custom: HttpCustom,
}

#[derive(Args, Clone, Debug)]
pub struct HttpCustom {
    #[arg(long, default_value_t = 8000)]
    pub http_port: u16,

    #[arg(long, default_value_t = 443)]
    pub https_port: u16,

    #[arg(long, value_name = "FILE")]
    https_key_path: Option<PathBuf>,

    #[arg(long, value_name = "FILE")]
    https_cert_path: Option<PathBuf>,
}

#[derive(Clone)]
pub struct AppState {
    pub context: Context,
    pub options: HttpOptions,
}

impl From<AppState> for LfsConfig {
    fn from(value: AppState) -> Self {
        Self {
            host: value.options.common.host,
            port: value.options.custom.http_port,
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

pub async fn start_server(config: Config, options: &HttpOptions) {
    let HttpOptions {
        common: CommonOptions { host },
        custom:
            HttpCustom {
                https_key_path: _,
                https_cert_path: _,
                http_port,
                https_port: _,
            },
    } = options;
    let server_url = format!("{}:{}", host, http_port);

    let state = AppState {
        options: options.to_owned(),
        context: Context::new(config.clone()).await,
    };

    let api_state = ApiServiceState {
        context: Context::new(config).await,
    };

    // add RequestDecompressionLayer for handle gzip encode
    // add TraceLayer for log record
    // add CorsLayer to add cors header
    let app = Router::new()
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
        .with_state(state);

    let addr = SocketAddr::from_str(&server_url).unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
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
