pub mod command;
pub mod indexer;
pub mod qdrant;
pub mod utils;
pub mod vectorization;

pub const QDRANT_URL: &str = "http://localhost:6334";
pub const VECT_URL: &str = "http://localhost:11434/api/embeddings";
pub const LLM_URL: &str = "http://localhost:11434/api/chat";
pub const PROCESS_ITEMS_NODE: &str = "process_items";
pub const VECT_CLIENT_NODE: &str = "vect_client";
pub const QDRANT_NODE: &str = "qdrant";
pub const SEARCH_NODE: &str = "search";
pub const GENERATION_NODE: &str = "generation";

pub const CONSUMER_GROUP: &str = "test-group";
pub const BROKER: &str = "kafka:9092";
pub const TOPIC: &str = "mega-crate-downloads-test";

pub const CRATES_PATH: &str = "/mnt/data/crates";
