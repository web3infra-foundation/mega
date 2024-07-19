use std::net::SocketAddr;
use std::str::FromStr;

use axum::body::{to_bytes, Body};
use axum::extract::{Query, State};
use axum::http::{Request, Response, StatusCode, Uri};
use axum::routing::get;
use axum::Router;
use common::config::Config;
use gemini::RelayGetParams;
use jupiter::context::Context;
use regex::Regex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use crate::api::api_router::{self};
use crate::api::ApiServiceState;

pub async fn run_ca_server(config: Config, _host: String, port: u16) {
    let host = "127.0.0.1".to_string();
    let app = app(config.clone(), host.clone(), port).await;

    let server_url = format!("{}:{}", host, port);
    tracing::info!("start ca server: {server_url}");
    let addr = SocketAddr::from_str(&server_url).unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[derive(Clone)]
pub struct AppState {
    pub context: Context,
    pub host: String,
    pub port: u16,
}

pub async fn app(config: Config, host: String, port: u16) -> Router {
    let state = AppState {
        host,
        port,
        context: Context::new(config.clone()).await,
    };

    let api_state = ApiServiceState {
        context: Context::new(config).await,
    };

    // add RequestDecompressionLayer for handle gzip encode
    // add TraceLayer for log record
    // add CorsLayer to add cors header
    Router::new()
        .nest("/api/", api_router::routers().with_state(api_state))
        .route(
            "/*path",
            get(get_method_router)
                .post(post_method_router)
                .delete(delete_method_router),
        )
        .layer(ServiceBuilder::new().layer(CorsLayer::new().allow_origin(Any)))
        .layer(TraceLayer::new_for_http())
        .layer(RequestDecompressionLayer::new())
        .with_state(state)
}

async fn get_method_router(
    _state: State<AppState>,
    Query(_params): Query<RelayGetParams>,
    uri: Uri,
) -> Result<Response<Body>, (StatusCode, String)> {
    if Regex::new(r"/certificates/[a-zA-Z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        let name = match gemini::ca::get_cert_name_from_path(uri.path()) {
            Some(n) => n,
            None => {
                return gemini::ca::response_error(
                    StatusCode::BAD_REQUEST.as_u16(),
                    "Bad request".to_string(),
                )
            }
        };
        return gemini::ca::get_certificate(name).await;
    }
    Err((
        StatusCode::NOT_FOUND,
        String::from("Operation not supported\n"),
    ))
}

async fn post_method_router(
    state: State<AppState>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let _ztm_config = state.context.config.ztm.clone();
    if Regex::new(r"/certificates/[a-zA-Z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        let name = match gemini::ca::get_cert_name_from_path(uri.path()) {
            Some(n) => n,
            None => {
                return gemini::ca::response_error(
                    StatusCode::BAD_REQUEST.as_u16(),
                    "Bad request".to_string(),
                )
            }
        };
        return gemini::ca::issue_certificate(name).await;
    } else if Regex::new(r"/sign/hub/[a-zA-Z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        let name = match gemini::ca::get_hub_name_from_path(uri.path()) {
            Some(n) => n,
            None => {
                return gemini::ca::response_error(
                    StatusCode::BAD_REQUEST.as_u16(),
                    "Bad request".to_string(),
                )
            }
        };
        let bytes = to_bytes(req.into_body(), usize::MAX).await.unwrap();
        let pubkey = String::from_utf8(bytes.to_vec()).unwrap();
        return gemini::ca::sign_certificate(name, pubkey).await;
    }
    Err((
        StatusCode::NOT_FOUND,
        String::from("Operation not supported\n"),
    ))
}

async fn delete_method_router(
    _state: State<AppState>,
    uri: Uri,
    _req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    if Regex::new(r"/certificates/[a-zA-Z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        return gemini::ca::delete_certificate(uri.path()).await;
    }
    Err((
        StatusCode::NOT_FOUND,
        String::from("Operation not supported\n"),
    ))
}

#[cfg(test)]
mod tests {}
