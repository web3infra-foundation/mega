// Orion worker client modules
mod api;
mod buck_controller;
mod util;
mod ws;

use uuid::Uuid;

#[tokio::main]
async fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(true)
        .init();

    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Configure WebSocket server address
    let server_addr =
        std::env::var("SERVER_WS").unwrap_or_else(|_| "ws://127.0.0.1:8004/ws".to_string());

    // Configure worker identification
    let worker_id = std::env::var("ORION_WORKER_ID").unwrap_or_else(|_| {
        tracing::warn!("ORION_WORKER_ID not set, generating a random worker ID for this session.");
        // Generate time-ordered UUID for better traceability
        Uuid::now_v7().to_string()
    });

    tracing::info!("Starting orion worker...");
    tracing::info!("  Worker ID: {}", worker_id);
    tracing::info!("  Connecting to server at: {}", server_addr);

    // Start WebSocket client with persistent connection
    ws::run_client(server_addr, worker_id).await;
}
