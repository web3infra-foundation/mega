pub mod command;
pub mod indexer;
pub mod qdrant;
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


pub fn consumer_group() -> String {
    std::env::var("CONSUMER_GROUP").unwrap_or_else(|_| "test-group".to_string())
}

pub fn broker() -> String {
    std::env::var("BROKER").unwrap_or_else(|_| "kafka:9092".to_string())
}

pub fn topic() -> String {
    std::env::var("TOPIC").unwrap_or_else(|_| "REPO_SYNC_STATUS.dev.0902".to_string())
}

pub fn crates_path() -> String {
    std::env::var("CRATES_PATH").unwrap_or_else(|_| "/opt/data/crates".to_string())
}
