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

pub fn consumer_group() -> String {
    std::env::var("CONSUMER_GROUP").unwrap_or_else(|_| "cve-consumer-serials".to_string())
}

pub fn broker() -> String {
    std::env::var("BROKER").unwrap_or_else(|_| "10.42.0.1:30092".to_string())
}

pub fn topic() -> String {
    std::env::var("TOPIC").unwrap_or_else(|_| "RAG.full.20251104".to_string())
}

