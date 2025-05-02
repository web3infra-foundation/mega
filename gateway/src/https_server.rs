use axum::routing::get;
use axum::{http, Router};
use clap::Args;

use quinn::rustls;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use common::model::{CommonHttpOptions, P2pOptions};
use jupiter::context::Context;
use mono::api::lfs::lfs_router;
use mono::api::MonoApiServiceState;
use mono::server::https_server::{get_method_router, post_method_router, AppState};

use crate::api::{github_router, MegaApiServiceState};

#[derive(Args, Clone, Debug)]
pub struct HttpOptions {
    #[clap(flatten)]
    pub common: CommonHttpOptions,

    #[clap(flatten)]
    pub p2p: P2pOptions,
}

pub async fn http_server(context: Context, options: HttpOptions) {
    let HttpOptions {
        common: CommonHttpOptions { host, port, .. },
        p2p,
    } = options.clone();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    check_run_with_p2p(context.clone(), options.p2p.clone());

    let app = app(context, host.clone(), port, p2p.clone()).await;

    let server_url = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(server_url).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub async fn app(context: Context, host: String, port: u16, p2p: P2pOptions) -> Router {
    let state = AppState {
        host: host.clone(),
        port,
        context: context.clone(),
    };

    let mono_api_state = MonoApiServiceState {
        context: context.clone(),
        oauth_client: None,
        store: None,
        listen_addr: format!("http://{}:{}", host, port),
    };

    let mega_api_state = MegaApiServiceState {
        inner: mono_api_state.clone(),
        p2p,
    };

    pub fn mega_routers() -> Router<MegaApiServiceState> {
        Router::new().merge(github_router::routers())
    }

    // add RequestDecompressionLayer for handle gzip encode
    // add TraceLayer for log record
    // add CorsLayer to add cors header
    Router::new()
        .merge(lfs_router::routers().with_state(mono_api_state.clone()))
        .merge(
            Router::new()
                .nest(
                    "/api/v1/mono",
                    mono::api::api_router::routers().with_state(mono_api_state.clone()),
                )
                .nest(
                    "/api/v1/mega",
                    mega_routers().with_state(mega_api_state.clone()),
                ),
        )
        // Using Regular Expressions for Path Matching in Protocol
        .route("/{*path}", get(get_method_router).post(post_method_router))
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

pub fn check_run_with_p2p(context: Context, p2p: P2pOptions) {
    //Mega server join a ztm mesh
    match p2p.bootstrap_node {
        Some(bootstrap_node) => {
            tracing::info!(
                "The bootstrap node is {}, prepare to join p2p network",
                bootstrap_node.clone()
            );

            tokio::spawn(async move { gemini::p2p::client::run(context, bootstrap_node).await });
        }
        None => {
            tracing::info!("The bootstrap node is not set, prepare to start mega server locally");
        }
    };
}

#[cfg(test)]
mod tests {}
