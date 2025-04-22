use clap::Parser;
use dagrs::utils::env::EnvVar;
use dagrs::{DefaultNode, Graph, Node, NodeTable};
use index::command::{Cli, Commands};
use index::indexer::CodeIndexer;
use index::indexer::ProcessItemsAction;
use index::indexer::WalkDirAction;
use index::kafka::get_consumer;
use index::qdrant::QdrantNode;
use index::vectorization::VectClient;
use index::{
    GENERATION_NODE, LLM_URL, PROCESS_ITEMS_NODE, QDRANT_NODE, QDRANT_URL, SEARCH_NODE,
    VECT_CLIENT_NODE, VECT_URL,
};
use rdkafka::consumer::CommitMode;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::Message;
use std::env;
use std::thread;
use tokio::runtime::Runtime;
use tokio_stream::StreamExt;

fn main() {
    env::set_var("RUST_LOG", "INFO");
    env_logger::init();

    let args = Cli::parse();

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let consumer = get_consumer();
        consume_messages(consumer, &args).await;
    });
}

async fn consume_messages(consumer: StreamConsumer, args: &Cli) {
    let mut message_stream = consumer.stream();

    while let Some(message) = message_stream.next().await {
        match message {
            Ok(m) => match m.payload_view::<str>() {
                Some(Ok(payload)) => {
                    println!("Received: {}", payload);
                    update_knowledge_base(args);
                    consumer.commit_message(&m, CommitMode::Async).unwrap();
                }
                Some(Err(e)) => eprintln!("UTF-8 error: {}", e),
                None => println!("Empty message"),
            },
            Err(e) => eprintln!("Kafka error: {}", e),
        }
    }
}

fn update_knowledge_base(args: &Cli) {
    log::info!("Start updating knowledge base...");
    let indexer = CodeIndexer::new(&args.workspace);
    log::info!("Start indexing directory: {:?}", indexer.crate_path);

    let mut index_node_table = NodeTable::default();
    let crate_version = "0.1.0";
    let walk_dir_action = WalkDirAction {
        indexer: indexer.clone(),
        crate_version: crate_version.to_owned(),
    };
    let process_items_action = ProcessItemsAction;

    let walk_dir = DefaultNode::with_action(
        "walk_dir".to_string(),
        walk_dir_action,
        &mut index_node_table,
    );
    let walk_dir_id = walk_dir.id();

    let process_items = DefaultNode::with_action(
        "process_items".to_string(),
        process_items_action,
        &mut index_node_table,
    );
    let process_items_id = process_items.id();

    let vect_client = VectClient::new(VECT_URL);
    let vect_client_node = DefaultNode::with_action(
        "vect_client".to_string(),
        vect_client,
        &mut index_node_table,
    );
    let vect_client_id = vect_client_node.id();

    let qdrant = QdrantNode::new(QDRANT_URL, "code_items");
    let qdrant_node = DefaultNode::with_action("qdrant".to_string(), qdrant, &mut index_node_table);
    let qdrant_id = qdrant_node.id();

    let mut index_graph = Graph::new();
    let mut index_env = EnvVar::new(index_node_table);

    index_graph.add_node(walk_dir);
    index_graph.add_node(process_items);
    index_graph.add_node(vect_client_node);
    index_graph.add_node(qdrant_node);

    index_graph.add_edge(walk_dir_id, vec![process_items_id]);
    index_graph.add_edge(process_items_id, vec![vect_client_id]);
    index_graph.add_edge(vect_client_id, vec![qdrant_id]);

    index_env.set(PROCESS_ITEMS_NODE, process_items_id);
    index_env.set(VECT_CLIENT_NODE, vect_client_id);
    index_env.set(QDRANT_NODE, qdrant_id);

    index_graph.set_env(index_env);

    // Use std::thread::spawn to handle blocking operations
    let handle = thread::spawn(move || {
        index_graph.start().unwrap();
    });

    handle.join().unwrap();
    log::info!("Knowledge base updated!");
}
