use super::{ScorpioManager, WorkDir};
use crate::manager::store::store_trees;
use crate::scolfs;
use crate::util::config;
use crate::util::GPath;
use async_recursion::async_recursion;
use ceres::model::git::LatestCommitInfo;
use crossbeam::queue::SegQueue;
use futures::future::join_all;
use git_internal::hash::SHA1;
use git_internal::internal::object::tree::{Tree, TreeItemMode};
use git_internal::internal::object::{
    commit::Commit,
    signature::{Signature, SignatureType},
};
use reqwest::Client;
use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::sync::watch;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::time;
use tokio::time::Duration;

///Download a file needs it's blob_id and save_path.
#[derive(Debug, Clone)]
pub struct DownloadTask {
    file_id: SHA1,
    save_path: PathBuf,
    retry_count: u32,
}

impl DownloadTask {
    pub fn new(file_id: SHA1, save_path: PathBuf) -> Self {
        Self {
            file_id,
            save_path,
            retry_count: 0,
        }
    }

    /// Create a retry task with incremented retry count
    pub fn retry(&self) -> Self {
        Self {
            file_id: self.file_id,
            save_path: self.save_path.clone(),
            retry_count: self.retry_count + 1,
        }
    }

    /// Check if the task has exceeded maximum retry attempts
    pub fn is_max_retries_exceeded(&self) -> bool {
        self.retry_count >= 3
    }
}

/// DownloadManager is responsible for managing file download operations in a concurrent manner.
///
/// ## File Download Flow:
/// 1. **Directory Processing Phase**:
///    - Directory workers traverse the file tree using BFS (Breadth-First Search)
///    - They create directories inline and enqueue file download tasks
///    - The `directory_processing_sender` tracks whether directory traversal is still ongoing
///
/// 2. **File Download Phase**:
///    - File download tasks are processed by a fixed pool of worker threads
///    - Each task downloads a file from the server using its SHA1 hash
///    - Workers update the `pending_tasks` counter as they complete downloads
///
/// 3. **Completion Coordination**:
///    - The system only considers the entire operation complete when:
///      a) Directory processing is finished (directory_processing_sender = false)
///      b) All file downloads are complete (pending_tasks = 0)
///    - A completion coordinator monitors both conditions and notifies waiters
///
/// ## Key Components:
/// - `sender`: Channel for enqueuing new download tasks
/// - `pending_tasks`: Atomic counter tracking active download operations
/// - `completion_notify`: Notifies when all operations are complete
/// - `directory_processing_sender/receiver`: Tracks directory traversal state
///
/// This design ensures that we don't prematurely consider downloads complete
/// while directories are still being processed and potentially creating new download tasks.
pub struct DownloadManager {
    sender: mpsc::UnboundedSender<DownloadTask>, // used to add new download tasks
    #[allow(unused)]
    worker_handles: Vec<tokio::task::JoinHandle<()>>,
    pending_tasks: Arc<AtomicUsize>, //the number of pending download tasks
    completion_notify: Arc<Notify>,  // used to notify when all tasks are done
    directory_processing_sender: watch::Sender<bool>, // watch the dir processing state
    directory_processing_receiver: watch::Receiver<bool>,
}

static DOWNLOAD_MANAGER: OnceLock<DownloadManager> = OnceLock::new();

impl DownloadManager {
    /// Creates a new DownloadManager with the specified number of worker threads.
    ///
    /// The manager starts with directory processing enabled (true) and spawns
    /// worker threads to handle file download tasks concurrently.
    pub fn new(worker_count: usize) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let pending_tasks = Arc::new(AtomicUsize::new(0));
        let completion_notify = Arc::new(Notify::new());

        let (directory_processing_sender, directory_processing_receiver) = watch::channel(true);

        let worker_handles = (0..worker_count)
            .map(|worker_id| {
                let receiver = receiver.clone();
                let sender = sender.clone();
                let pending_tasks = pending_tasks.clone();
                let completion_notify = completion_notify.clone();
                let directory_receiver = directory_processing_receiver.clone();
                tokio::spawn(async move {
                    Self::worker_loop(
                        worker_id,
                        receiver,
                        sender,
                        pending_tasks,
                        completion_notify,
                        directory_receiver,
                    )
                    .await;
                })
            })
            .collect();

        Self {
            sender,
            worker_handles,
            pending_tasks,
            completion_notify,
            directory_processing_sender,
            directory_processing_receiver,
        }
    }

    /// Starts the completion coordinator to handle task completion notifications.
    ///
    /// The coordinator monitors directory processing state changes and automatically
    /// notifies waiters when both directory processing is complete AND no files are pending.
    pub fn start_completion_coordinator(&self) {
        let completion_notify = self.completion_notify.clone();
        let pending_tasks = self.pending_tasks.clone();
        let mut directory_receiver = self.directory_processing_receiver.clone();

        tokio::spawn(async move {
            loop {
                if directory_receiver.changed().await.is_err() {
                    break;
                }

                let directory_processing = *directory_receiver.borrow();
                let has_pending = pending_tasks.load(Ordering::Relaxed) > 0;

                if !directory_processing && !has_pending {
                    completion_notify.notify_waiters();
                    break;
                }
            }
        });
    }

    /// Worker loop that processes download tasks from the queue.
    ///
    /// Each worker continuously:
    /// 1. Waits for download tasks from the shared queue
    /// 2. Downloads files using fetch_and_save_file()
    /// 3. If download fails and retries are available, re-enqueues the task
    /// 4. Updates the pending task counter only on success or max retries exceeded
    /// 5. Notifies completion when it's the last task AND directory processing is done
    async fn worker_loop(
        worker_id: usize,
        receiver: Arc<Mutex<mpsc::UnboundedReceiver<DownloadTask>>>,
        sender: mpsc::UnboundedSender<DownloadTask>,
        pending_tasks: Arc<AtomicUsize>,
        completion_notify: Arc<Notify>,
        directory_receiver: watch::Receiver<bool>,
    ) {
        loop {
            let task = {
                let mut rx = receiver.lock().await;
                rx.recv().await
            };

            match task {
                Some(task) => {
                    match fetch_and_save_file(&task.file_id, &task.save_path).await {
                        Ok(_) => {
                            // Download successful, proceed to decrement counter
                        }
                        Err(e) => {
                            if task.is_max_retries_exceeded() {
                                eprintln!(
                                    "Worker {}: Failed to download file {} (path: {}) after {} retries, giving up: {}",
                                    worker_id, task.file_id, task.save_path.display(), task.retry_count, e
                                );
                                // Max retries exceeded, proceed to decrement counter
                            } else {
                                eprintln!(
                                    "Worker {}: Failed to download file {} (path: {}) on attempt {}, retrying: {}",
                                    worker_id, task.file_id, task.save_path.display(), task.retry_count + 1, e
                                );

                                // Create retry task and re-enqueue
                                let retry_task = task.retry();
                                if let Err(retry_err) = sender.send(retry_task) {
                                    eprintln!(
                                        "Worker {}: Failed to re-enqueue retry task for file {} (path: {}): {}",
                                        worker_id, task.file_id, task.save_path.display(), retry_err
                                    );
                                    // If we can't re-enqueue, we still need to decrement the counter
                                } else {
                                    // Successfully re-enqueued, don't decrement counter
                                    continue;
                                }
                            }
                        }
                    }

                    let remaining = pending_tasks.fetch_sub(1, Ordering::Relaxed);

                    if remaining == 1 {
                        let directory_processing = *directory_receiver.borrow();
                        if !directory_processing {
                            completion_notify.notify_waiters();
                        }
                    }
                }
                None => {
                    break;
                }
            }
        }
    }

    /// Enqueue a file download task to be processed by worker threads.
    ///
    /// This increments the pending task counter and sends the task to workers.
    /// Returns an error if the channel is closed.
    pub fn enqueue_download(&self, task: DownloadTask) -> Result<(), String> {
        self.pending_tasks.fetch_add(1, Ordering::Relaxed);
        self.sender
            .send(task)
            .map_err(|_| "Failed to enqueue download task".to_string())
    }

    pub fn has_pending_tasks(&self) -> bool {
        self.pending_tasks.load(Ordering::Relaxed) > 0
    }

    /// Notifies that directory processing has completed.
    ///
    /// This is called when all directory traversal workers have finished.
    /// If no files are pending, it immediately notifies completion.
    pub fn notify_directory_processing_complete(&self) {
        let _ = self.directory_processing_sender.send(false);

        if !self.has_pending_tasks() {
            self.completion_notify.notify_waiters();
        }
    }

    /// Returns true if directory processing is still ongoing.
    pub fn is_directory_processing(&self) -> bool {
        *self.directory_processing_receiver.borrow()
    }

    /// Waits for all operations to complete using an efficient notification system.
    ///
    /// This method waits until BOTH conditions are met:
    /// 1. Directory processing is complete (no more directories being traversed)
    /// 2. All file downloads are complete (pending_tasks == 0)
    ///
    /// Uses tokio::select! to efficiently wait for state changes rather than busy polling.
    pub async fn wait_for_completion(&self) {
        loop {
            let directory_processing = self.is_directory_processing();
            let has_pending = self.has_pending_tasks();

            if !directory_processing && !has_pending {
                return;
            }

            let mut directory_receiver = self.directory_processing_receiver.clone();

            tokio::select! {
                _ = self.completion_notify.notified() => {
                    continue;
                }
                _ = directory_receiver.changed() => {
                    continue;
                }
                // _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                //     continue;
                // }
            }
        }
    }

    pub fn get_global() -> &'static DownloadManager {
        DOWNLOAD_MANAGER.get_or_init(|| {
            let worker_count = config::fetch_file_thread();
            println!("Initializing global download manager with {worker_count} workers");
            DownloadManager::new(worker_count)
        })
    }
}

/// Enqueue a file download task,the common entry point for downloading files.
pub fn enqueue_file_download(file_id: SHA1, save_path: PathBuf) {
    let download_manager = DownloadManager::get_global();
    let task = DownloadTask::new(file_id, save_path);

    if let Err(e) = download_manager.enqueue_download(task) {
        eprintln!("Failed to enqueue download task for file {file_id}: {e}");
    }
}

/// Download multiple files for CL scenarios.
pub async fn download_cl_files(
    files: Vec<(SHA1, PathBuf)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let download_manager = DownloadManager::get_global();

    // Since this is for CL files only (no directory traversal),
    // we mark directory processing as complete immediately
    download_manager.notify_directory_processing_complete();

    // Enqueue all file download tasks
    for (file_id, save_path) in files {
        let task = DownloadTask::new(file_id, save_path);
        if let Err(e) = download_manager.enqueue_download(task) {
            return Err(format!("Failed to enqueue download task for file {file_id}: {e}").into());
        }
    }

    // Wait for all downloads to complete
    download_manager.wait_for_completion().await;

    Ok(())
}

#[allow(async_fn_in_trait)]
pub trait CheckHash {
    async fn check(&mut self);

    async fn fetch<P: AsRef<Path> + std::marker::Send>(
        &mut self,
        inode: u64,
        monopath: P,
    ) -> WorkDir;
}

impl CheckHash for ScorpioManager {
    async fn check(&mut self) {
        let mut handlers = Vec::new();

        for work in &mut self.works {
            // if the config hash is null or empty , mean that it's a new config work node path .
            if work.hash.is_empty() {
                let p = GPath::from(work.path.to_string());
                // Get the tree and its hash value, for name dictionary .
                let tree = fetch_tree(&p).await.unwrap();
                work.hash = tree.id.to_string();
                // the lower path is store file path for remote code version .
                let store_path = config::store_path();
                let _lower = PathBuf::from(store_path).join(&work.hash).join("lower");
                handlers.push(tokio::spawn(async move { fetch_code(&p, _lower).await }));
            }
        }
        // if have new config path , finish all handlers and write back the config file
        if !handlers.is_empty() {
            for handle in handlers {
                let _ = handle.await;
            }
            //Get config file path from scorpio_config.rs
            let config_file = config::config_file();
            let _ = self.to_toml(config_file);
        }
    }

    #[allow(unused)]
    async fn fetch<P: AsRef<Path> + std::marker::Send>(
        &mut self,
        inode: u64,
        monopath: P,
    ) -> WorkDir {
        let path = monopath.as_ref().to_str().unwrap().to_string();
        let p = GPath::from(path);
        // Get the tree and its hash value, for name dictionary .
        let tree = fetch_tree(&p).await.unwrap();
        let workdir = WorkDir {
            path: p.to_string(),
            node: inode,
            hash: tree.id.to_string(),
        };
        //work.hash = tree.id.to_string();
        // the lower path is store file path for remote code version .
        let store_path = config::store_path();
        let _lower = PathBuf::from(store_path).join(&workdir.hash).join("lower");
        fetch_code(&p, _lower).await.unwrap();
        self.works.push(workdir.clone());
        let config_file = config::config_file();
        let _ = self.to_toml(config_file);

        workdir
    }
}

/// The core function of fetch operation
pub async fn fetch<P: AsRef<Path>>(
    manager: &mut ScorpioManager,
    inode: u64,
    monopath: P,
    orion_path: &str,
) -> std::io::Result<WorkDir> {
    let path = monopath.as_ref().to_str().unwrap().to_string();
    let p = GPath::from(path);
    let o = GPath::from(orion_path.to_string());
    // Get the tree and its hash value, for name dictionary .
    let tree = fetch_tree(&p).await.unwrap();
    let workdir = WorkDir {
        path: p.to_string(),
        node: inode,
        hash: tree.id.to_string(),
    };
    //work.hash = tree.id.to_string();
    // the lower path is store file path for remote code version .
    let store_path = config::store_path();
    let work_path = PathBuf::from(store_path).join(&workdir.hash);
    let _lower = work_path.join("lower");
    fetch_code(&o, _lower).await?;
    manager.works.push(workdir.clone());
    let config_file = config::config_file();
    let _ = manager.to_toml(config_file);

    // Get the commit information of the previous version and
    // write it into the commit file.
    // set_parent_commit(&work_path, orion_path).await?;

    Ok(workdir)
}

#[allow(unused)]
#[allow(clippy::blocks_in_conditions)]
async fn worker_thread(
    id: u32,
    root_path: GPath,
    target_path: &Path,
    shared_queue: Arc<Mutex<VecDeque<GPath>>>,
    send_tree: Sender<Tree>,
) {
    let client = Client::new();
    //let mut interval = time::interval(Duration::from_millis(50));
    let timeout_duration = Duration::from_millis(300);
    loop {
        let path = tokio::select! {
            _ = time::sleep(timeout_duration) => {
                // If timeout and no more tree, finish this thread.
                println!("Timeout occurred while waiting for path");
                break;
            },
            path = async {
                loop{
                    {
                        let mut queue = shared_queue.lock().await;
                        if let Some(pa) = queue.pop_front(){
                            break pa;
                        }
                    }
                }
            } => {
                path
            }
        };
        // deal with  path .
        let url = format!("{}{}", config::tree_file_endpoint(), path);
        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes().await {
                        Ok(bytes) => {
                            match Tree::try_from(&bytes[..]) {
                                Ok(tree) => {
                                    trace!("ID:{id},path:{path}");
                                    send_tree.send(tree.clone()).await;
                                    //trace!("path:{},new tree:{}",path,tree );
                                    for item in tree.tree_items {
                                        let mut subpath = path.clone(); // New path ->  mono/repo/dirpath
                                        subpath.push(item.name);
                                        let real_path = target_path.join(
                                            subpath.part(root_path.path.len(), subpath.path.len()),
                                        );
                                        if item.mode == TreeItemMode::Tree {
                                            {
                                                let mut queue = shared_queue.lock().await;
                                                queue.push_back(subpath);
                                            }
                                            // mkdir
                                            tokio::fs::create_dir_all(real_path).await.unwrap();
                                        } else {
                                            // TODO: fetch file and save to target path. about file fetch api, refer to test_fetch_octet_stream() test func.
                                            fetch_and_save_file(&item.id, real_path).await.unwrap();
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("Failed to parse tree: {e:?}");
                                }
                            }
                        }
                        Err(e) => {
                            println!("Failed to get response bytes: {e:?}");
                        }
                    }
                } else {
                    println!("Failed to fetch tree: {}", response.status());
                }
            }
            Err(e) => {
                println!("Failed to send request: {e:?}");
            }
        }
    }
}

/// Network operations, recursively read the main Tree from the pipe, and download Blobs objects
#[async_recursion]
#[allow(unused)]
async fn worker_ro_thread(
    root_path: GPath,
    target_path: Arc<PathBuf>,
    path: GPath,
    send_tree: Sender<(GPath, Tree)>,
) {
    let tree = fetch_tree(&path).await.unwrap();
    trace!("path:{path}");
    let _ = send_tree.send((path.clone(), tree.clone())).await;
    let mut handlers = Vec::new();
    //trace!("path:{},new tree:{}",path,tree );
    for item in tree.tree_items {
        let mut subpath = path.clone(); // New path ->  mono/repo/dirpath
        subpath.push(item.name);
        let real_path = target_path.join(subpath.part(root_path.path.len(), subpath.path.len()));
        if item.mode == TreeItemMode::Tree {
            {
                let root_path = root_path.clone();
                let _path = target_path.clone();
                let send_tree = send_tree.clone();
                handlers.push(tokio::spawn(async move {
                    worker_ro_thread(root_path, _path, subpath, send_tree.clone()).await
                }));
            }
            // mkdir
            tokio::fs::create_dir_all(real_path).await.unwrap();
        } else {
            let e = fetch_and_save_file(&item.id, real_path).await;
            println!("{e:?}");
        }
    }
    for h in handlers {
        let _ = h.await;
    }
}

///
/// the tree info is store in k-v database.
///     monorepo path  -> Tree
///
/// Download remote data to local and store it in Overlay format
async fn fetch_code(path: &GPath, save_path: impl AsRef<Path>) -> std::io::Result<()> {
    let target_path = save_path.as_ref().to_path_buf();

    let download_manager = DownloadManager::get_global();
    download_manager.start_completion_coordinator();
    let _ = download_manager.directory_processing_sender.send(true);

    // Create the save_path directory if it doesn't exist
    tokio::fs::create_dir_all(&save_path).await?;

    // Setup tree storage channel
    let (tree_sender, tree_receiver) = mpsc::channel::<(GPath, Tree)>(1000);

    // Use simple queue for directory processing, similar to load_dir_depth
    let queue = Arc::new(SegQueue::new());

    // Fetch and process the initial/root directory
    let initial_tree = fetch_tree(path).await.map_err(std::io::Error::other)?;

    // Send tree to storage
    if let Err(e) = tree_sender.send((path.clone(), initial_tree.clone())).await {
        eprintln!("Failed to send initial tree: {e}");
    }

    let dir_count = initial_tree
        .tree_items
        .iter()
        .filter(|item| item.mode == TreeItemMode::Tree)
        .count();

    let active_producers = Arc::new(AtomicUsize::new(dir_count));

    for item in initial_tree.tree_items {
        let item_name = item.name.clone();
        let real_path = target_path.join(&item_name);

        if item.mode == TreeItemMode::Tree {
            // Create directory and add to queue for processing
            if let Err(e) = tokio::fs::create_dir_all(&real_path).await {
                eprintln!("Failed to create directory {real_path:?}: {e}");
                // If we failed to create directory, reduce the producer count
                active_producers.fetch_sub(1, Ordering::Release);
                continue;
            }

            let mut subpath = path.clone();
            subpath.push(item_name);
            queue.push((subpath, real_path));
        } else {
            // Enqueue file for download
            enqueue_file_download(item.id, real_path);
        }
    }

    let worker_count = 5;
    let mut workers = Vec::with_capacity(worker_count);

    for worker_id in 0..worker_count {
        let queue = Arc::clone(&queue);
        let tree_sender = tree_sender.clone();
        let producers = Arc::clone(&active_producers);

        workers.push(tokio::spawn(async move {
            while producers.load(Ordering::Acquire) > 0 || !queue.is_empty() {
                if let Some((current_path, current_target)) = queue.pop() {
                    // Fetch tree for this directory
                    match fetch_tree(&current_path).await {
                        Ok(tree) => {
                            // Send tree to storage
                            if let Err(e) =
                                tree_sender.send((current_path.clone(), tree.clone())).await
                            {
                                eprintln!("Worker {worker_id}: Failed to send tree: {e}");
                            }

                            // Process each item in the tree
                            for item in tree.tree_items {
                                let item_name = item.name.clone();
                                let item_real_path = current_target.join(&item_name);

                                if item.mode == TreeItemMode::Tree {
                                    // Create directory
                                    if let Err(_e) =
                                        tokio::fs::create_dir_all(&item_real_path).await
                                    {
                                        continue;
                                    }

                                    // Add subdirectory to queue and increment producer count
                                    let mut subpath = current_path.clone();
                                    subpath.push(item_name);
                                    producers.fetch_add(1, Ordering::Release);
                                    queue.push((subpath, item_real_path));
                                } else {
                                    // Enqueue file for download
                                    enqueue_file_download(item.id, item_real_path);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Worker {worker_id}: Failed to fetch tree for {current_path}: {e}",
                            );
                        }
                    }

                    producers.fetch_sub(1, Ordering::Release);
                } else {
                    if producers.load(Ordering::Acquire) == 0 {
                        return;
                    }
                    tokio::task::yield_now().await;
                }
            }
            // println!("Worker {worker_id} shutting down");
        }));
    }

    // Drop tree sender to allow storage to finish
    drop(tree_sender);

    let storepath = save_path.as_ref().parent().unwrap().join("tree.db");
    let store_handle = tokio::spawn(async move {
        if let Err(e) = store_trees(storepath.to_str().unwrap(), tree_receiver).await {
            eprintln!("Failed to store trees: {e}");
        }
    });

    // Wait for all workers to complete
    join_all(workers).await;

    DownloadManager::get_global().notify_directory_processing_complete();
    // println!("Directory processing completed for {path}");

    let _ = store_handle.await;

    // Wait for all downloads to complete
    DownloadManager::get_global().wait_for_completion().await;

    // Get LFS files
    scolfs::lfs::lfs_restore(&path.to_string(), save_path.as_ref().to_str().unwrap())
        .await
        .unwrap();

    println!("Finished downloading code for {path}");
    Ok(())
}

/// Get the previous version of the Commit information from the remote API,
/// convert it into a Commit structure, and write it into the commit file.
async fn _set_parent_commit(work_path: &Path, repo_path: &str) -> std::io::Result<()> {
    let parent_commit = match fetch_parent_commit(repo_path).await {
        Ok(info) => info,
        Err(e) => {
            eprintln!("Failed to fetch parent commit info: {e}");
            return Err(std::io::Error::other("Failed to fetch parent commit info"));
        }
    };

    let path = work_path.join("commit");
    let mut commit_file = std::fs::File::create(&path)?;
    commit_file.write_all(parent_commit.to_string().as_bytes())?;

    Ok(())
}

// async fn fetch_code_nore(path:&GPath, save_path : impl AsRef<Path>){

//         let queue = Arc::new(Mutex::new(VecDeque::new()));
//         let queue_clone = queue.clone();

//         let p = path.clone();
//         let _handle = tokio::spawn(async move {
//             // Initialize the queue with a path
//             let mut queue = queue_clone.lock().await;
//             queue.push_back( p);
//         });
//         let mut handles = vec![];
//         let target_path: Arc<PathBuf> = Arc::new(save_path.as_ref().to_path_buf());

//         // Create the save_path directory if it doesn't exist
//         tokio::fs::create_dir_all(&save_path).await.unwrap();
//         let rece;
//         {
//             let (s,r) = tokio::sync::mpsc::channel::<Tree>(100);
//             rece = r;
//             for i in 0..10 {
//                 let p: GPath = path.clone();
//                 let queue_clone = queue.clone();
//                 let o = target_path.clone();
//                 let ss = s.clone();
//                 let handle = tokio::spawn(async move {
//                     worker_thread(i, p, &o, queue_clone,ss).await;
//                 });
//                 handles.push(handle);
//             }
//         }

//         // Clean up workers (depends on how you implement worker_thread termination)
//         for handle in handles {
//             let _ = handle.await;
//         }
//         let storepath = save_path.as_ref().parent().unwrap().join("tree.db");
//         store::store_trees(storepath.to_str().unwrap(), rece).await;
//         // Check if the queue has been populated
//         let queue = queue.lock().await;
//         assert!(queue.len() == 0);

// }

/// Network operations, extracting Blobs objects from HTTP byte streams and storing them
async fn fetch_and_save_file(
    url: &SHA1,
    save_path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let file_blob_endpoint = config::file_blob_endpoint();
    let url = format!("{file_blob_endpoint}/{url}");
    // Send GET request
    let response = client.get(url).send().await?;

    // Ensure that the response status is successful
    if response.status().is_success() {
        // Get the binary data from the response body
        let content = response.bytes().await?;

        // Store the content in a Vec<u8>
        let data: Vec<u8> = content.to_vec();

        // Save the data to a file
        tokio::fs::write(save_path, data).await?;
    } else {
        eprintln!("Request failed with status: {}", response.status());
    }

    Ok(())
}

/// Network operations, extracting Tree objects from HTTP byte streams
#[allow(unused)]
pub async fn fetch_tree(path: &GPath) -> Result<Tree, String> {
    let url = format!("{}{}", config::tree_file_endpoint(), path);
    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if response.status().is_success() {
        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read response: {e}"))?;
        let tree = Tree::try_from(&bytes[..]).map_err(|e| format!("Failed to parse tree: {e}"))?;
        Ok(tree)
    } else {
        Err(format!("Failed to fetch tree: {}", response.status()))
    }
}

/// Network operations, extracting parent commit Hash from HTTP byte streams
pub async fn fetch_parent_commit(path: &str) -> Result<Commit, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/api/v1/latest-commit?path=/{}",
        config::base_url(),
        path.trim_start_matches('/')
    );
    let response = reqwest::get(&url).await?;

    if response.status().is_success() {
        let parent_info = response.json::<LatestCommitInfo>().await?;
        let author_sign = Signature::new(
            SignatureType::Author,
            parent_info.author.clone(),
            String::new(),
        );
        let committer_sign = Signature::new(
            SignatureType::Committer,
            parent_info.committer.clone(),
            String::new(),
        );
        Ok(Commit::new(
            author_sign,
            committer_sign,
            SHA1::from_str(&parent_info.oid)?,
            Vec::new(),
            &parent_info.short_message,
        ))
    } else {
        Err(format!("Failed to fetch tree: {}", response.status()).into())
    }
}

#[cfg(test)]
mod tests {
    use git_internal::internal::object::tree::Tree;
    use reqwest::Client;
    use std::error::Error;
    use std::fs::File;
    #[tokio::test]
    #[ignore = "requires running Mega server (uses base_url from config)"]
    async fn test_fetch_octet_stream() -> Result<(), Box<dyn Error>> {
        // Create an HTTP client
        let client = Client::new();

        // Use base_url from config (e.g., http://git.gitmega.com)
        let url = format!(
            "{}/api/v1/file/tree?path=/third-party/mega",
            crate::util::config::base_url()
        );

        // Send GET request
        let response = client.get(url).send().await?;

        // Ensure that the response status is successful
        if response.status().is_success() {
            // Get the binary data from the response body
            let content = response.bytes().await?;

            // Store the content in a Vec<u8>
            let data: Vec<u8> = content.to_vec();
            let tree = Tree::try_from(&data[..]).unwrap();
            // Print the data length for testing assertions
            // println!("Received {} bytes of data", data.len());

            // // You can add more assertions or validation logic here
            // assert!(!data.is_empty(), "Data should not be empty");

            println!("{tree}");
            // You can also validate the specific content of the data
            // assert_eq!(data, expected_data); // You need to define expected_data
        } else {
            eprintln!("Request failed with status: {}", response.status());
            return Err(format!("Request failed with status: {}", response.status()).into());
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires running Mega server (uses base_url from config)"]
    async fn test_fetch_octet_file() {
        // Create an HTTP client
        let client = Client::new();

        // Use base_url from config (e.g., http://git.gitmega.com)
        let url = format!(
            "{}/api/v1/file/blob/841b6fe34540e866e1f458d77b1bd03d3cb0e782",
            crate::util::config::base_url()
        );
        // Send a GET request
        let response = client.get(url).send().await.unwrap();

        // Ensure that the response status is successful
        if response.status().is_success() {
            // Get the binary data from the response body
            let content = response.bytes().await.unwrap();

            // Store the content in a Vec<u8>
            let data: Vec<u8> = content.to_vec();
            use std::io::prelude::*;
            // Save the data to a file
            let mut file = File::create("output.txt").unwrap();
            file.write_all(&data).unwrap();

            // Print the path to the saved file
            println!("Data saved to output.txt");
        } else {
            eprintln!("Request failed with status: {}", response.status());
        }
    }
}
