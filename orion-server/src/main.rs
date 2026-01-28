

mod api;
mod auto_retry;
mod buck2;
mod orion_common;
mod log;
mod model;
mod scheduler;
mod server;

/// Orion Build Server
/// A distributed build system that manages build tasks and worker nodes
#[tokio::main]
async fn main() {
    // Initialize logging with DEBUG level
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Load environment variables from .env file (optional)
    dotenvy::dotenv().ok();

    // // Get server port from environment or use default
    // let port: u16 = std::env::var("PORT")
    //     .unwrap_or_else(|_| "8004".to_string())
    //     .parse()
    //     .expect("PORT must be a number");

    server::start_server().await;
}


