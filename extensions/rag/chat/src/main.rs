use chat::command::{Cli, Commands};
use chat::generation::GenerationNode;
use chat::search::SearchNode;
use chat::{GENERATION_NODE, LLM_URL, QDRANT_URL, SEARCH_NODE, VECT_URL};
use clap::Parser;
use dagrs::utils::env::EnvVar;
use dagrs::{DefaultNode, Graph, Node, NodeTable};
use std::env;
use std::thread;

fn main() {
    env::set_var("RUST_LOG", "INFO");
    env_logger::init();

    let args = Cli::parse();

    match args.command {
        Commands::Chat => {
            log::info!("Start the conversation process...");
            let mut search_node_table = NodeTable::default();

            let search_node = SearchNode::new(VECT_URL, QDRANT_URL, "code_items");
            let search_node = DefaultNode::with_action(
                SEARCH_NODE.to_string(),
                search_node,
                &mut search_node_table,
            );
            let search_id = search_node.id();

            let generation_node = GenerationNode::new(LLM_URL);
            let generation_node = DefaultNode::with_action(
                GENERATION_NODE.to_string(),
                generation_node,
                &mut search_node_table,
            );
            let generation_id = generation_node.id();

            let mut search_graph = Graph::new();
            let mut search_env = EnvVar::new(search_node_table);

            search_graph.add_node(search_node);
            search_graph.add_node(generation_node);

            search_graph.add_edge(search_id, vec![generation_id]);

            search_env.set(SEARCH_NODE, search_id);
            search_env.set(GENERATION_NODE, generation_id);

            search_graph.set_env(search_env);

            // Use std: Thread:: spawn to handle blocking operations
            let handle = thread::spawn(move || {
                search_graph.start().unwrap();
            });

            handle.join().unwrap();
        }
    }
}
