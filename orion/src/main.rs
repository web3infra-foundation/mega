mod server;
mod buck_controller;
mod api;
mod util;
mod model;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt() // default is INFO
        .with_max_level(tracing::Level::DEBUG)
        .init();
    tracing::info!("current dir: {:?}", std::env::current_dir().unwrap());
    dotenvy::dotenv().unwrap();

    server::start_server(8001).await;
}
