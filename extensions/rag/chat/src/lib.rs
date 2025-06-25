pub mod command;
pub mod generation;
pub mod search;
pub mod utils;
pub mod vectorization;

pub const QDRANT_URL: &str = "http://172.17.0.1:6334";
pub const VECT_URL: &str = "http://ollama:11434/api/embeddings";
pub const LLM_URL: &str = "http://ollama:11434/api/chat";
pub const PROCESS_ITEMS_NODE: &str = "process_items";
pub const VECT_CLIENT_NODE: &str = "vect_client";
pub const QDRANT_NODE: &str = "qdrant";
pub const SEARCH_NODE: &str = "search";
pub const GENERATION_NODE: &str = "generation";

pub const RAG_OUTPUT: &str = "output.json";
