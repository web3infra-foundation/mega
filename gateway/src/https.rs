//!
//!
//!
//!
//!
use std::collections::HashMap;
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
use git::lfs::LfsConfig;
use regex::Regex;
use serde::Deserialize;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use common::enums::DataSource;
use git::protocol::{PackProtocol, Protocol, ServiceType};
use storage::driver::database;
use storage::driver::database::storage::ObjectStorage;

use crate::{lfs, git_http};

/// Parameters for starting the HTTP service
#[derive(Args, Clone, Debug)]
pub struct HttpOptions {
    /// Server start hostname
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub host: String,

    #[arg(short, long, default_value_t = 8000)]
    pub port: u16,

    #[arg(short, long, value_name = "FILE")]
    key_path: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    cert_path: Option<PathBuf>,

    #[arg(short, long, value_enum, default_value = "postgres")]
    pub data_source: DataSource,
}

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn ObjectStorage>,
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

pub async fn http_server(options: &HttpOptions) -> Result<(), Box<dyn std::error::Error>> {
    let HttpOptions {
        host,
        port,
        key_path: _,
        cert_path: _,
        data_source,
    } = options;
    let server_url = format!("{}:{}", host, port);

    let state = AppState {
        storage: database::init(data_source).await,
        options: options.to_owned(),
    };
    let app = Router::new()
        .nest("/api/v1", api_routers::routers(state.clone()))
        .route(
            "/*path",
            get(get_method_router)
                .post(post_method_router)
                .put(put_method_router),
        )
        .layer(ServiceBuilder::new().layer(CorsLayer::new().allow_origin(Any)))
        .with_state(state);

    let addr = SocketAddr::from_str(&server_url).unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
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
        return lfs::lfs_download_object(&lfs_config, uri.path()).await;
    } else if Regex::new(r"/locks$").unwrap().is_match(uri.path()) {
        return lfs::lfs_retrieve_lock(&lfs_config, params).await;
    }

    let service_name = params.service.unwrap();
    let service_type = service_name.parse::<ServiceType>().unwrap();

    // # Discovering Reference
    // HTTP clients that support the "smart" protocol (or both the "smart" and "dumb" protocols) MUST
    // discover references by making a parameterized request for the info/refs file of the repository.
    // The request MUST contain exactly one query parameter, service=$servicename,
    // where $servicename MUST be the service name the client wishes to contact to complete the operation.
    // The request MUST NOT contain additional query parameters.
    if !Regex::new(r"/info/refs$").unwrap().is_match(uri.path()) {
        return Err((
            StatusCode::FORBIDDEN,
            String::from("Operation not supported\n"),
        ));
    }
    let mut pack_protocol = PackProtocol::new(
        remove_git_suffix(uri, "/info/refs"),
        state.storage.clone(),
        Protocol::Http,
    );
    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        format!("application/x-{}-advertisement", service_name),
    );
    headers.insert(
        "Cache-Control".to_string(),
        "no-cache, max-age=0, must-revalidate".to_string(),
    );
    tracing::info!("headers: {:?}", headers);
    let mut resp = Response::builder();
    for (key, val) in headers {
        resp = resp.header(&key, val);
    }

    let pkt_line_stream = pack_protocol.git_info_refs(service_type).await;
    let body = Body::from(pkt_line_stream.freeze());
    Ok(resp.body(body).unwrap())
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
        return lfs::lfs_verify_lock(state, &lfs_config, req).await;
    } else if Regex::new(r"/locks$").unwrap().is_match(uri.path()) {
        return lfs::lfs_create_lock(state, &lfs_config, req).await;
    } else if Regex::new(r"/unlock$").unwrap().is_match(uri.path()) {
        return lfs::lfs_delete_lock(state, &lfs_config, uri.path(), req).await;
    } else if Regex::new(r"/objects/batch$").unwrap().is_match(uri.path()) {
        return lfs::lfs_process_batch(state, &lfs_config, req).await;
    }

    if Regex::new(r"/git-upload-pack$")
        .unwrap()
        .is_match(uri.path())
    {
        let pack_protocol = PackProtocol::new(
            remove_git_suffix(uri, "/git-upload-pack"),
            state.storage.clone(),
            Protocol::Http,
        );
        git_http::git_upload_pack(req, pack_protocol).await
    } else if Regex::new(r"/git-receive-pack$")
        .unwrap()
        .is_match(uri.path())
    {
        let pack_protocol = PackProtocol::new(
            remove_git_suffix(uri, "/git-receive-pack"),
            state.storage.clone(),
            Protocol::Http,
        );
        git_http::git_receive_pack(req, pack_protocol).await
    } else {
        Err((
            StatusCode::FORBIDDEN,
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
            StatusCode::FORBIDDEN,
            String::from("Operation not supported"),
        ))
    }
}

mod api_routers {
    use std::collections::HashMap;

    use axum::{
        extract::{Query, State},
        http::StatusCode,
        response::IntoResponse,
        routing::get,
        Json, Router,
    };

    use crate::{
        api_service::obj_service::ObjectService,
        model::{
            object_detail::{BlobObjects, Directories},
            query::DirectoryQuery,
        },
    };

    use crate::AppState;

    pub fn routers<S>(state: AppState) -> Router<S> {
        Router::new()
            .route("/blob", get(get_blob_object))
            .route("/tree", get(get_directories))
            .route("/object", get(get_origin_object))
            .with_state(state)
    }

    async fn get_blob_object(
        Query(query): Query<HashMap<String, String>>,
        state: State<AppState>,
    ) -> Result<Json<BlobObjects>, (StatusCode, String)> {
        let object_id = query.get("object_id").unwrap();
        let object_service = ObjectService {
            storage: state.storage.clone(),
        };
        object_service.get_blob_objects(object_id).await
    }

    async fn get_directories(
        Query(query): Query<DirectoryQuery>,
        state: State<AppState>,
    ) -> Result<Json<Directories>, (StatusCode, String)> {
        let object_service = ObjectService {
            storage: state.storage.clone(),
        };
        object_service.get_directories(query).await
    }

    async fn get_origin_object(
        Query(query): Query<HashMap<String, String>>,
        state: State<AppState>,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let object_id = query.get("object_id").unwrap();
        let object_service = ObjectService {
            storage: state.storage.clone(),
        };
        object_service.get_objects_data(object_id).await
    }
}

#[cfg(test)]
mod tests {}
