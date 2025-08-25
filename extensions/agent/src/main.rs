mod api;
mod model;

use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let app = api::router();

    let listener = TcpListener::bind("0.0.0.0:8800").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
