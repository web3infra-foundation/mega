use dagrs::utils::env::EnvVar;
use dagrs::{DefaultNode, Graph, Node, NodeTable};
use index::indexer::CodeIndexer;
use index::indexer::ProcessItemsAction;
use index::indexer::WalkDirAction;
use index::qdrant::QdrantNode;
use index::vectorization::VectClient;
use index::{BROKER, CONSUMER_GROUP, CRATES_PATH, TOPIC};
use index::{PROCESS_ITEMS_NODE, QDRANT_NODE, QDRANT_URL, VECT_CLIENT_NODE, VECT_URL};
use observatory::facilities::Telescope;
use observatory::model::crates::CrateMessage;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use tokio::runtime::Runtime;

fn get_file_path(crates_path: &Path, c_name: &str, c_version: &str) -> PathBuf {
    crates_path
        .join(c_name)
        .join(format!("{}-{}.crate", c_name, c_version))
}

fn main() {
    env::set_var("RUST_LOG", "INFO");
    env_logger::init();

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let telescope = Telescope::new(BROKER, CONSUMER_GROUP, TOPIC);
        telescope
            .consume_loop(|payload: String| async move {
                println!("✅ Received: {}", payload);

                let crate_msg = serde_json::from_str::<CrateMessage>(&payload).unwrap();
                let file_path = get_file_path(
                    &PathBuf::from(CRATES_PATH),
                    &crate_msg.crate_name,
                    &crate_msg.crate_version,
                );
                println!("📦 File path: {:?}", file_path);

                tokio::spawn(async move {
                    tokio::task::spawn_blocking(move || {
                        update_knowledge_base(&file_path);
                    })
                    .await
                    .unwrap();
                });
            })
            .await;
    });
}

fn update_knowledge_base(file_path: &PathBuf) {
    log::info!("Start updating knowledge base...");
    let indexer = CodeIndexer::new(file_path);
    log::info!("Start indexing directory: {:?}", indexer.crate_path);

    let mut index_node_table = NodeTable::default();
    let walk_dir_action = WalkDirAction {
        indexer: indexer.clone(),
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

    let qdrant = QdrantNode::new(QDRANT_URL, "test_test_code_items");
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
