use axum::Router;
use axum::routing::get;
use crate::api;

pub async fn start_server(port: u16) {
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .merge(api::routers());

    tracing::info!("Listening on port {}", port);

    let addr = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", port)).await.unwrap();
    axum::serve(addr, app.into_make_service()).await.unwrap();
}