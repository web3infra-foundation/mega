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
    // dotenvy::dotenv().ok(); // .env file is optional

    // let port: u16 = std::env::var("PORT")
    //     .unwrap_or_else(|_| "8001".to_string())
    //     .parse()
    //     .expect("PORT must be a number");
    // server::start_server(port).await;

    spawn_client("ws://localhost:8004/ws").await;
}
