mod api;
mod auto_retry;
mod buck2;
mod log;
mod model;
mod orion_common;
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

    server::start_server().await;
}
