use std::sync::Arc;

use axum::routing::get;
use axum::{Router, http};
use clap::Args;

use context::AppContext;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use common::model::CommonHttpOptions;
use jupiter::storage::Storage;
use mono::api::MonoApiServiceState;
use mono::api::lfs::lfs_router;
use mono::server::http_server::{ProtocolApiState, get_method_router, post_method_router};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::api::{MegaApiServiceState, github_router};

#[derive(Args, Clone, Debug)]
pub struct HttpOptions {
    #[clap(flatten)]
    pub common: CommonHttpOptions,
}

pub async fn http_server(context: AppContext, options: HttpOptions) {
    let HttpOptions {
        common: CommonHttpOptions { host, port, .. },
    } = options.clone();

    let app = app(context.storage, host.clone(), port).await;

    let server_url = format!("{host}:{port}");

    let listener = tokio::net::TcpListener::bind(server_url).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub async fn app(storage: Storage, host: String, port: u16) -> Router {
    let state = ProtocolApiState {
        storage: storage.clone(),
        shared: Arc::new(Mutex::new(0)),
    };

    let mono_api_state = MonoApiServiceState {
        storage: storage.clone(),
        oauth_client: None,
        session_store: None,
        listen_addr: format!("http://{host}:{port}"),
    };

    let mega_api_state = MegaApiServiceState {
        inner: mono_api_state.clone(),
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

#[derive(OpenApi)]
struct ApiDoc;

#[cfg(test)]
mod tests {}
