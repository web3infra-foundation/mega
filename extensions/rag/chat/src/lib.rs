pub mod command;
pub mod generation;
pub mod search;
pub mod utils;
pub mod vectorization;

use std::env;

use crate::generation::GenerationNode;
use crate::search::SearchNode;

pub const PROCESS_ITEMS_NODE: &str = "process_items";
pub const VECT_CLIENT_NODE: &str = "vect_client";
pub const QDRANT_NODE: &str = "qdrant";
pub const SEARCH_NODE: &str = "search";
pub const GENERATION_NODE: &str = "generation";

pub fn qdrant_url() -> String {
    env::var("QDRANT_URL").unwrap_or_else(|_| "http://172.17.0.1:6334".to_string())
}

pub fn vect_url() -> String {
    env::var("VECT_URL").unwrap_or_else(|_| "http://ollama:11434/api/embeddings".to_string())
}

pub fn llm_url() -> String {
    env::var("LLM_URL").unwrap_or_else(|_| "http://ollama:11434/api/chat".to_string())
}

pub const RAG_OUTPUT: &str = "output.json";

pub async fn search_context(query: &str) -> Result<String, Box<dyn std::error::Error>> {
    let search_node = SearchNode::new(&vect_url(), &qdrant_url(), "test_test_code_items")?;
    let context = match search_node.search(query).await? {
        Some((content, _item_type)) => content,
        None => query.to_string(),
    };
    Ok(context)
}

pub async fn generate_suggestion(context: &str) -> Result<String, Box<dyn std::error::Error>> {
    let generation_node = GenerationNode::new(&llm_url());
    let result = generation_node.generate(context).await?;
    Ok(result)
}

pub async fn chat_response(query: &str) -> Result<String, Box<dyn std::error::Error>> {
    let context = search_context(query).await?;
    generate_suggestion(&context).await
}
