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
use git::protocol::http;
use git::protocol::{PackProtocol, Protocol};
use hyper::{Body, Request, StatusCode, Uri};
use regex::Regex;
use serde::Deserialize;
use storage::driver::{mysql, ObjectStorage};

/// Parameters for starting the HTTP service
#[derive(Args, Clone, Debug)]
pub struct HttpOptions {
    /// Server start hostname
    #[arg(long, default_value_t = String::from("0.0.0.0"))]
    host: String,

    #[arg(short, long, default_value_t = 8000)]
    port: u16,

    #[arg(short, long, value_name = "FILE")]
    key_path: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    cert_path: Option<PathBuf>,
}

#[derive(Clone)]
struct AppState {
    storage: Arc<dyn ObjectStorage>,
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
    } = options;
    let server_url = format!("{}:{}", host, port);

    let state = AppState {
        storage: Arc::new(mysql::init().await),
    };

    let app = Router::new()
        .route("/*path", get(git_info_refs).post(data_transfer))
        .with_state(state);

    let addr = SocketAddr::from_str(&server_url).unwrap();
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}
/// QueryParameters ServiceName
#[derive(Deserialize, Debug)]
struct ServiceName {
    pub service: String,
}

/// Discovering Reference
async fn git_info_refs(
    state: State<AppState>,
    Query(service): Query<ServiceName>,
    uri: Uri,
) -> Result<Response<Body>, (StatusCode, String)> {
    let service_name = service.service;

    if !Regex::new(r"/info/refs$").unwrap().is_match(uri.path()) {
        return Err((
            StatusCode::FORBIDDEN,
            String::from("Operation not supported"),
        ));
    }
    if service_name == "git-upload-pack" || service_name == "git-receive-pack" {
        let mut pack_protocol = PackProtocol::new(
            remove_git_suffix(uri, "/info/refs"),
            &service_name,
            state.storage.clone(),
            Protocol::Http,
        );
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            format!(
                "application/x-{}-advertisement",
                pack_protocol.service_type.unwrap().to_string()
            ),
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

        let pkt_line_stream = pack_protocol.git_info_refs().await;
        let body = Body::from(pkt_line_stream.freeze());
        Ok(resp.body(body).unwrap())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            String::from("Operation not supported"),
        ))
    }
}

async fn data_transfer(
    state: State<AppState>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    if Regex::new(r"/git-upload-pack$")
        .unwrap()
        .is_match(uri.path())
    {
        git_upload_pack(state, remove_git_suffix(uri, "/git-upload-pack"), req).await
    } else if Regex::new(r"/git-receive-pack$")
        .unwrap()
        .is_match(uri.path())
    {
        git_receive_pack(state, remove_git_suffix(uri, "/git-receive-pack"), req).await
    } else {
        Err((
            StatusCode::FORBIDDEN,
            String::from("Operation not supported"),
        ))
    }
}

/// Smart Service git-upload-pack, handle git pull and clone
async fn git_upload_pack(
    state: State<AppState>,
    path: PathBuf,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let pack_protocol = PackProtocol::new(path, "", state.storage.clone(), Protocol::Http);

    http::git_upload_pack(req, pack_protocol).await
}

// http://localhost:8000/org1/apps/App2.git
// http://localhost:8000/org1/libs/lib1.git
/// Smart Service git-receive-pack, handle git push
async fn git_receive_pack(
    state: State<AppState>,
    path: PathBuf,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("req: {:?}", req);
    let pack_protocol = PackProtocol::new(path, "", state.storage.clone(), Protocol::Http);
    http::git_receive_pack(req, pack_protocol).await
}

#[cfg(test)]
mod tests {}
