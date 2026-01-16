use std::env;

use axum::{routing::post, Json, Router};
use chat::{generation::GenerationNode, llm_url, qdrant_url, search::SearchNode, vect_url};
use log::{error, info};
use serde::Deserialize;

#[derive(Deserialize)]
struct ChatRequest {
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Logger
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    // Build Axum app
    let app = Router::new().route("/chat", post(chat_handler));

    info!("Server running on http://0.0.0.0:30088");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:30088").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// POST /chat
async fn chat_handler(Json(payload): Json<ChatRequest>) -> Result<Json<String>, String> {
    info!("Received chat request: {}", payload.prompt);

    // Create SearchNode with request prompt
    let search_node = SearchNode::new(
        &vect_url(),
        &qdrant_url(),
        "test_test_code_items",
        &payload.prompt,
    )
    .expect("Failed to create SearchNode");

    // Execute search directly
    let search_result = match search_node.search(&payload.prompt).await {
        Ok(Some((content, item_type))) => {
            info!(
                "Search result found: type={}, content length={}",
                item_type,
                content.len()
            );
            info!("Search content: {}", content);
            format!(
                "{}\nThe enhanced information after local RAG may be helpful, but it is not necessarily accurate:\n Related information type: {}\nRelated information Content: {}",
                payload.prompt,
                item_type,
                content
            )
        }
        Ok(None) => {
            info!("No search results found");
            payload.prompt
        }
        Err(e) => {
            error!("Search error: {}", e);
            return Err(format!("Search failed: {}", e));
        }
    };

    info!("Search result for generation: {}", search_result);

    // Create GenerationNode and execute generation
    let generation_node = GenerationNode::new(&llm_url(), None); // No oneshot needed for direct execution
    let generated_message = match generation_node.generate(&search_result).await {
        Ok(msg) => {
            info!("Generation completed successfully");
            msg
        }
        Err(e) => {
            error!("Generation error: {}", e);
            return Err(format!("Generation failed: {}", e));
        }
    };

    info!("Final response: {}", generated_message);
    Ok(Json(generated_message))
}
