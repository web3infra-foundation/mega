use std::net::SocketAddr;
use std::str::FromStr;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{Response, StatusCode, Uri};
use axum::routing::get;
use axum::Router;
use clap::Args;
use common::config::Config;
use common::model::{CommonOptions, GetParams};
use jupiter::context::Context;
use regex::Regex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use crate::api_service;
use crate::api_service::router::ApiServiceState;

#[derive(Args, Clone, Debug)]
pub struct RelayOptions {
    #[clap(flatten)]
    pub common: CommonOptions,

    #[arg(long, default_value_t = 8001)]
    pub http_port: u16,
}

pub async fn http_server(config: Config, options: RelayOptions) {
    let RelayOptions {
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
        .nest(
            "/api/v1",
            api_service::router::routers().with_state(api_state),
        )
        .route(
            "/*path",
            get(get_method_router), // .post(post_method_router)
                                    // .put(put_method_router),
        )
        .layer(ServiceBuilder::new().layer(CorsLayer::new().allow_origin(Any)))
        .layer(TraceLayer::new_for_http())
        .layer(RequestDecompressionLayer::new())
        .with_state(state)
}

async fn get_method_router(
    _state: State<AppState>,
    Query(params): Query<GetParams>,
    uri: Uri,
) -> Result<Response<Body>, (StatusCode, String)> {
    if Regex::new(r"/hello$").unwrap().is_match(uri.path()) {
        return gemini::http::handler::hello_gemini(params).await;
    }
    return Err((
        StatusCode::NOT_FOUND,
        String::from("Operation not supported\n"),
    ));
}

#[cfg(test)]
mod tests {}
