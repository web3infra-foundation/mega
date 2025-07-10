use dagrs::utils::env::EnvVar;
use dagrs::{DefaultNode, Graph, Node, NodeTable};
use index::indexer::CodeIndexer;
use index::indexer::ProcessItemsAction;
use index::indexer::WalkDirAction;
use index::qdrant::QdrantNode;
use index::vectorization::VectClient;
use index::{broker, consumer_group, crates_path, topic};
use index::{PROCESS_ITEMS_NODE, QDRANT_NODE, VECT_CLIENT_NODE, qdrant_url, vect_url};
use observatory::facilities::Telescope;
use observatory::model::crates::CrateMessage;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
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

    dotenv::from_path("extensions/rag/.env").ok();

    // 1. Initialize the shared atomic counter once at the start.
    let id_path = "/opt/data/last_id.json";
    let initial_id = fs::read_to_string(id_path)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let shared_id_counter = Arc::new(AtomicU64::new(initial_id));

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let telescope = Telescope::new(&broker(), &consumer_group(), &topic());
        
        // 1. Clone the Arc *before* the loop's closure.
        // This clone is moved into the closure, allowing the original to remain.
        let id_counter_for_loop = Arc::clone(&shared_id_counter);

        telescope
            .consume_loop(move |payload: String| {
                // 2. Clone the Arc again *inside* the closure for each task.
                // This ensures each spawned task gets its own reference.
                let id_counter_for_task = Arc::clone(&id_counter_for_loop);
                async move {
                    println!("✅ Received: {}", payload);

                    let crate_msg = serde_json::from_str::<CrateMessage>(&payload).unwrap();
                    let file_path = get_file_path(
                        &PathBuf::from(crates_path()),
                        &crate_msg.crate_name,
                        &crate_msg.crate_version,
                    );
                    println!("📦 File path: {:?}", file_path);

                    tokio::spawn(async move {
                        tokio::task::spawn_blocking(move || {
                            update_knowledge_base(&file_path, id_counter_for_task);
                        })
                        .await
                        .unwrap();
                    });
                }
            })
            .await;
    });
}

fn update_knowledge_base(file_path: &PathBuf, id_counter: Arc<AtomicU64>) {
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

    let vect_client = VectClient::new(&vect_url());
    let vect_client_node = DefaultNode::with_action(
        "vect_client".to_string(),
        vect_client,
        &mut index_node_table,
    );
    let vect_client_id = vect_client_node.id();

    // 3. Pass the shared counter to the QdrantNode constructor.
    let qdrant = QdrantNode::new(&qdrant_url(), "test_test_code_items", id_counter);
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
