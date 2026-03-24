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

    orion_server::server::start_server().await;
}
