pub mod command;
pub mod generation;
pub mod search;
pub mod utils;
pub mod vectorization;

use std::env;

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
