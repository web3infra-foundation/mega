use chat::command::{Cli, Commands};
use chat::generation::GenerationNode;
use chat::search::SearchNode;
use chat::{llm_url, qdrant_url, vect_url, GENERATION_NODE, SEARCH_NODE};
use clap::Parser;
use dagrs::utils::env::EnvVar;
use dagrs::{DefaultNode, Graph, Node, NodeTable};
use log::{error, info};
use std::env;
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    dotenv::from_path("extensions/rag/.env").ok();

    let args = Cli::parse();

    match args.command {
        Commands::Chat => {
            info!("Starting the conversation process...");

            // Create node table
            let mut search_node_table = NodeTable::default();

            // Safely create SearchNode
            let search_node = match SearchNode::new(&vect_url(), &qdrant_url(), "code_items") {
                Ok(node) => node,
                Err(e) => {
                    error!("Failed to create SearchNode: {}", e);
                    return Err(e);
                }
            };

            let search_node = DefaultNode::with_action(
                SEARCH_NODE.to_string(),
                search_node,
                &mut search_node_table,
            );
            let search_id = search_node.id();

            // Create GenerationNode
            let generation_node = GenerationNode::new(&llm_url());
            let generation_node = DefaultNode::with_action(
                GENERATION_NODE.to_string(),
                generation_node,
                &mut search_node_table,
            );
            let generation_id = generation_node.id();

            // Build the graph
            let mut search_graph = Graph::new();
            let mut search_env = EnvVar::new(search_node_table);

            search_graph.add_node(search_node);
            search_graph.add_node(generation_node);
            search_graph.add_edge(search_id, vec![generation_id]);

            search_env.set(SEARCH_NODE, search_id);
            search_env.set(GENERATION_NODE, generation_id);
            search_graph.set_env(search_env);

            // Use thread to handle blocking operations
            let handle = thread::spawn(move || {
                if let Err(e) = search_graph.start() {
                    error!("Error executing search graph: {}", e);
                }
            });

            // Wait for the thread to finish
            if let Err(e) = handle.join() {
                error!("Thread panicked: {:?}", e);
                return Err("Thread execution failed".into());
            }

            info!("Conversation process completed successfully");
        }
    }

    Ok(())
}
