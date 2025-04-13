use crate::ws::spawn_client;

mod api;
mod buck_controller;
mod util;
mod ws;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt() // default is INFO
        .with_max_level(tracing::Level::DEBUG)
        .init();
    tracing::info!("current dir: {:?}", std::env::current_dir().unwrap());
    dotenvy::dotenv().ok(); // .env file is optional

    let server_ws = std::env::var("SERVER_WS").expect("SERVER_WS not set");
    spawn_client(&server_ws).await;
}
