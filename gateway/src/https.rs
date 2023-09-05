//!
//!
//!
//!
//!

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;

use axum::extract::{Query, State};
use axum::response::Response;
use axum::routing::get;
use axum::{Router, Server};
use clap::Args;
use database::driver::lfs::structs::LockListQuery;
use database::driver::ObjectStorage;
use database::DataSource;
use git::lfs::{self, LfsConfig};
use git::protocol::{http, ServiceType};
use git::protocol::{PackProtocol, Protocol};
use hyper::{Body, Request, StatusCode, Uri};
use regex::Regex;
use serde::Deserialize;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

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

    #[arg(short, long, default_value_os_t = PathBuf::from("lfs_content"))]
    pub lfs_content_path: PathBuf,

    #[arg(short, long, value_enum, default_value = "postgres")]
    pub data_source: DataSource,
}

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn ObjectStorage>,
    pub options: HttpOptions,
}

#[derive(Deserialize, Debug)]
struct GetParams {
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
        lfs_content_path: _,
        data_source,
    } = options;
    let server_url = format!("{}:{}", host, port);

    // let config =  LfsConfig::from(options.to_owned());
    let state = AppState {
        storage:database::init(data_source).await,
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
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

async fn get_method_router(
    state: State<AppState>,
    Query(params): Query<GetParams>,
    uri: Uri,
) -> Result<Response<Body>, (StatusCode, String)> {
    let mut lfs_config: LfsConfig = state.options.clone().into();
    lfs_config.storage = state.storage.clone();

    // Routing LFS services.
    if Regex::new(r"/objects/[a-z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        // Retrieve the `:oid` field from path.
        let path = uri.path().to_owned();
        let tokens: Vec<&str> = path.split('/').collect();
        // The `:oid` field is the last field.
        return lfs::http::lfs_download_object(&lfs_config, tokens[tokens.len() - 1]).await;
    } else if Regex::new(r"/locks$").unwrap().is_match(uri.path()) {
        // Load query parameters into struct.
        let lock_list_query = LockListQuery {
            path: params.path,
            id: params.id,
            cursor: params.cursor,
            limit: params.limit,
            refspec: params.refspec,
        };
        return lfs::http::lfs_retrieve_lock(&lfs_config, lock_list_query).await;
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
) -> Result<Response<Body>, (StatusCode, String)> {
    let mut lfs_config: LfsConfig = state.options.clone().into();
    lfs_config.storage = state.storage.clone();

    // Routing LFS services.
    if Regex::new(r"/locks/verify$").unwrap().is_match(uri.path()) {
        return lfs::http::lfs_verify_lock(&lfs_config, req).await;
    } else if Regex::new(r"/locks$").unwrap().is_match(uri.path()) {
        return lfs::http::lfs_create_lock(&lfs_config, req).await;
    } else if Regex::new(r"/unlock$").unwrap().is_match(uri.path()) {
        // Retrieve the `:id` field from path.
        let path = uri.path().to_owned();
        let tokens: Vec<&str> = path.split('/').collect();
        // The `:id` field is just ahead of the last field.
        return lfs::http::lfs_delete_lock(&lfs_config, tokens[tokens.len() - 2], req).await;
    } else if Regex::new(r"/objects/batch$").unwrap().is_match(uri.path()) {
        return lfs::http::lfs_process_batch(&lfs_config, req).await;
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
        http::git_upload_pack(req, pack_protocol).await
    } else if Regex::new(r"/git-receive-pack$")
        .unwrap()
        .is_match(uri.path())
    {
        let pack_protocol = PackProtocol::new(
            remove_git_suffix(uri, "/git-receive-pack"),
            state.storage.clone(),
            Protocol::Http,
        );
        http::git_receive_pack(req, pack_protocol).await
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
    let mut lfs_config: LfsConfig = state.options.clone().into();
    lfs_config.storage = state.storage.clone();
    if Regex::new(r"/objects/[a-z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        // Retrieve the `:oid` field from path.
        let path = uri.path().to_owned();
        let tokens: Vec<&str> = path.split('/').collect();
        // The `:oid` field is the last field.
        lfs::http::lfs_upload_object(&lfs_config, tokens[tokens.len() - 1], req).await
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
        response::IntoResponse,
        routing::get,
        Json, Router,
    };
    use hyper::StatusCode;

    use crate::{
        api_service::obj_service::ObjectService,
        model::object_detail::{BlobObjects, TreeObjects},
    };

    use super::AppState;

    pub fn routers<S>(state: AppState) -> Router<S> {
        Router::new()
            .route("/blob", get(get_blob_object))
            .route("/tree", get(get_tree_objects))
            .route("/object", get(get_origin_object))
            .with_state(state)
    }

    async fn get_blob_object(
        Query(query): Query<HashMap<String, String>>,
        state: State<AppState>,
    ) -> Result<Json<BlobObjects>, (StatusCode, String)> {
        let repo_path = query.get("repo_path").unwrap();
        let object_id = query.get("object_id").unwrap();
        let object_service = ObjectService {
            storage: state.storage.clone(),
        };
        object_service.get_blob_objects(object_id, repo_path).await
    }

    async fn get_tree_objects(
        Query(query): Query<HashMap<String, String>>,
        state: State<AppState>,
    ) -> Result<Json<TreeObjects>, (StatusCode, String)> {
        let object_id = query.get("object_id");
        let repo_path = query.get("repo_path").unwrap();
        let object_service = ObjectService {
            storage: state.storage.clone(),
        };
        object_service.get_tree_objects(object_id, repo_path).await
    }

    async fn get_origin_object(
        Query(query): Query<HashMap<String, String>>,
        state: State<AppState>,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let repo_path = query.get("repo_path").unwrap();
        let object_id = query.get("object_id").unwrap();
        let object_service = ObjectService {
            storage: state.storage.clone(),
        };
        object_service.get_objects_data(object_id, repo_path).await
    }
}

#[cfg(test)]
mod tests {}
