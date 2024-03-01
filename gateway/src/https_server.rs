//!
//!
//!
//!
//!
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{Request, StatusCode, Uri};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use clap::Args;

use jupiter::storage::init::database_connection;
use jupiter::storage::mega_storage::MegaStorage;
use regex::Regex;
use serde::Deserialize;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use common::model::CommonOptions;
use git::lfs::LfsConfig;
use git::protocol::{PackProtocol, Protocol};
use storage::driver::database;
use storage::driver::database::storage::ObjectStorage;
use tower_http::trace::TraceLayer;

use crate::api_service::obj_service::ObjectService;
use crate::api_service::router::ApiServiceState;
use crate::{api_service, git_protocol, lfs};

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
    pub storage: Arc<dyn ObjectStorage>,
    pub mega_storage: MegaStorage,
    pub options: HttpOptions,
}

#[derive(Deserialize, Debug)]
pub struct GetParams {
    pub service: Option<String>,
    pub refspec: Option<String>,
    pub id: Option<String>,
    pub path: Option<String>,
    pub limit: Option<String>,
    pub cursor: Option<String>,
}

pub fn remove_git_suffix(uri: Uri, git_suffix: &str) -> PathBuf {
    PathBuf::from(uri.path().replace(".git", "").replace(git_suffix, ""))
}

pub async fn start_server(options: &HttpOptions) {
    let HttpOptions {
        common: CommonOptions { host, data_source },
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
        storage: database::init(data_source).await,
        options: options.to_owned(),
        mega_storage: MegaStorage::new(database_connection().await).await,
    };
    
    let api_state = ApiServiceState {
        object_service: ObjectService {
            storage: state.storage.clone(),
        },
        mega_storage: state.mega_storage.clone(),
    };
    
    let app = Router::new()
        .nest("/api/v1", api_service::router::routers(api_state))
        .route(
            "/*path",
            get(get_method_router)
                .post(post_method_router)
                .put(put_method_router),
        )
        .layer(ServiceBuilder::new().layer(CorsLayer::new().allow_origin(Any)))
        .layer(TraceLayer::new_for_http())
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
    let mut lfs_config: LfsConfig = state.deref().to_owned().into();
    lfs_config.fs_storage = storage::driver::file_storage::init("lfs-files".to_owned()).await;
    // Routing LFS services.
    if Regex::new(r"/objects/[a-z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        lfs::lfs_download_object(&lfs_config, uri.path()).await
    } else if Regex::new(r"/locks$").unwrap().is_match(uri.path()) {
        return lfs::lfs_retrieve_lock(&lfs_config, params).await;
    } else if Regex::new(r"/info/refs$").unwrap().is_match(uri.path()) {
        let pack_protocol = PackProtocol::new(
            remove_git_suffix(uri, "/info/refs"),
            state.storage.clone(),
            Protocol::Http,
        );
        return git_protocol::http::git_info_refs(params, pack_protocol).await;
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
    let mut lfs_config: LfsConfig = state.deref().to_owned().into();
    lfs_config.fs_storage = storage::driver::file_storage::init("lfs-files".to_owned()).await;
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
        let pack_protocol = PackProtocol::new(
            remove_git_suffix(uri, "/git-upload-pack"),
            state.storage.clone(),
            Protocol::Http,
        );
        git_protocol::http::git_upload_pack(req, pack_protocol).await
    } else if Regex::new(r"/git-receive-pack$")
        .unwrap()
        .is_match(uri.path())
    {
        let pack_protocol = PackProtocol::new(
            remove_git_suffix(uri, "/git-receive-pack"),
            state.storage.clone(),
            Protocol::Http,
        );
        git_protocol::http::git_receive_pack(req, pack_protocol).await
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
    let mut lfs_config: LfsConfig = state.deref().to_owned().into();
    lfs_config.fs_storage = storage::driver::file_storage::init("lfs-files".to_owned()).await;
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
