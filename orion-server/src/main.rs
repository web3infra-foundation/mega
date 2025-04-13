mod api;
mod model;
mod server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt() // default is INFO
        .with_max_level(tracing::Level::DEBUG)
        .init();

    dotenvy::dotenv().ok(); // .env file is optional
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8004".to_string())
        .parse()
        .expect("PORT must be a number");
    server::start_server(port).await;
}
