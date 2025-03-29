use crate::api::AppState;
use axum::Router;
use axum::routing::get;
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

mod api;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt() // default is INFO
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let port = 8004;
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .merge(api::routers())
        .with_state(AppState {
            clients: Arc::new(DashMap::new()),
        })
        // logging so we can see what's going on
        .layer(TraceLayer::new_for_http());

    tracing::info!("Listening on port {}", port);

    let addr = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    axum::serve(
        addr,
        app.into_make_service_with_connect_info::<SocketAddr>(), // or `ConnectInfo<SocketAddr>` fail
    )
    .await
    .unwrap();
}
