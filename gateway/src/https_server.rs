use axum::routing::get;
use axum::{http, Router};
use clap::Args;

use context::AppContext;
use quinn::rustls;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use common::model::{CommonHttpOptions, P2pOptions};
use jupiter::storage::Storage;
use mono::api::lfs::lfs_router;
use mono::api::MonoApiServiceState;
use mono::server::https_server::{get_method_router, post_method_router, AppState};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::api::{github_router, MegaApiServiceState};

#[derive(Args, Clone, Debug)]
pub struct HttpOptions {
    #[clap(flatten)]
    pub common: CommonHttpOptions,

    #[clap(flatten)]
    pub p2p: P2pOptions,
}

pub async fn http_server(context: AppContext, options: HttpOptions) {
    let HttpOptions {
        common: CommonHttpOptions { host, port, .. },
        p2p,
    } = options.clone();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    check_run_with_p2p(context.clone(), options.p2p.clone());

    let app = app(context.storage, host.clone(), port, p2p.clone()).await;

    let server_url = format!("{host}:{port}");

    let listener = tokio::net::TcpListener::bind(server_url).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub async fn app(storage: Storage, host: String, port: u16, p2p: P2pOptions) -> Router {
    let state = AppState {
        host: host.clone(),
        port,
        storage: storage.clone(),
    };

    let mono_api_state = MonoApiServiceState {
        storage: storage.clone(),
        oauth_client: None,
        store: None,
        listen_addr: format!("http://{host}:{port}"),
    };

    let mega_api_state = MegaApiServiceState {
        inner: mono_api_state.clone(),
        p2p,
    };

    pub fn mega_routers() -> OpenApiRouter<MegaApiServiceState> {
        OpenApiRouter::new().merge(github_router::routers())
    }

    // add RequestDecompressionLayer for handle gzip encode
    // add TraceLayer for log record
    // add CorsLayer to add cors header
    let (router, _) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(lfs_router::routers().with_state(mono_api_state.clone()))
        .merge(
            OpenApiRouter::new()
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
        .split_for_parts();
    router
}

pub fn check_run_with_p2p(context: AppContext, p2p: P2pOptions) {
    //Mega server join a ztm mesh
    match p2p.bootstrap_node {
        Some(bootstrap_node) => {
            tracing::info!(
                "The bootstrap node is {}, prepare to join p2p network",
                bootstrap_node.clone()
            );

            let client = context.client.wrapped_client();
            tokio::spawn(async move {
                if let Err(e) = client.run(bootstrap_node).await {
                    tracing::error!("P2P client closed:{}", e)
                }
            });
        }
        None => {
            tracing::info!("The bootstrap node is not set, prepare to start mega server locally");
        }
    };
}

#[derive(OpenApi)]
struct ApiDoc;

#[cfg(test)]
mod tests {}
