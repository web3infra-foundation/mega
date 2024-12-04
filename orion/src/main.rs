mod server;
mod buck_controller;
mod api;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt() // default is INFO
        .with_max_level(tracing::Level::DEBUG)
        .init();

    server::start_server(8001).await;
}
