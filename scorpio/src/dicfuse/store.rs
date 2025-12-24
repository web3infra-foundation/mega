use crate::READONLY_INODE;
use async_recursion::async_recursion;
/// Read only file system for obtaining and displaying monorepo directory information
use core::panic;
use crossbeam::queue::SegQueue;
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use futures::future::join_all;
use once_cell::sync::Lazy;
use reqwest::Client;
use rfuse3::raw::reply::ReplyEntry;
use rfuse3::FileType;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

use super::abi::{default_dic_entry, default_file_entry};
use super::content_store::ContentStorage;
use super::size_store::SizeStorage;
use super::tree_store::{StorageItem, TreeStorage};
use crate::util::{config, GPath};

/// Git SHA1 for an empty blob (0-byte file).
///
/// This lets us distinguish legitimate empty files from failures (e.g., network/HTTP errors)
/// that must NOT be cached as empty content.
pub(crate) const EMPTY_BLOB_OID: &str = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";
const UNKNOW_INODE: u64 = 0; // illegal inode number;
const INODE_FILE: &str = "file";
const INODE_DICTIONARY: &str = "directory";

static GLOBAL_IMPORT_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn global_import_semaphore() -> Arc<Semaphore> {
    GLOBAL_IMPORT_SEMAPHORE
        .get_or_init(|| Arc::new(Semaphore::new(config::dicfuse_import_concurrency())))
        .clone()
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Item {
    pub name: String,
    pub path: String,
    pub content_type: String,
}
impl Item {
    pub fn is_dir(&self) -> bool {
        self.content_type == INODE_DICTIONARY
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
//use hash to get the dir's status.
pub struct ItemExt {
    pub item: Item,
    pub hash: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct TreeInfoResponse {
    req_result: bool,
    data: Vec<TreeInfo>,
    err_message: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct TreeInfo {
    oid: String,
    name: String,
    content_type: String,
}

#[allow(unused)]
pub struct DicItem {
    inode: u64,
    path_name: GPath,
    content_type: Arc<Mutex<ContentType>>,
    pub children: Mutex<HashMap<String, Arc<DicItem>>>,
    parent: u64,
}

#[allow(unused)]
#[derive(PartialEq, Debug)]
enum ContentType {
    File,
    Directory(bool), // if this dictionary is loaded.
}
#[allow(unused)]
impl DicItem {
    pub fn new(inode: u64, parent: u64, item: Item) -> Self {
        DicItem {
            inode,
            path_name: item.path.into(), // GPath can be created from String
            content_type: match item.content_type.as_str() {
                INODE_FILE => Arc::new(Mutex::new(ContentType::File)),
                INODE_DICTIONARY => Arc::new(Mutex::new(ContentType::Directory(false))),
                _ => panic!("Unknown content type"),
            },
            children: Mutex::new(HashMap::new()),
            parent,
        }
    }
    //get the total path
    pub fn get_path(&self) -> String {
        self.path_name.to_string()
    }
    //get the file or dic name . aka tail name.
    pub fn get_name(&self) -> String {
        self.path_name.name()
    }
    // add a children item
    pub async fn push_children(&self, children: Arc<DicItem>) {
        self.children
            .lock()
            .await
            .insert(children.get_name(), children);
    }
    // get the inode
    pub fn get_inode(&self) -> u64 {
        self.inode
    }
    async fn get_tyep(&self) -> ContentType {
        let t = self.content_type.lock().await;
        match *t {
            ContentType::File => ContentType::File,
            ContentType::Directory(a) => ContentType::Directory(a),
        }
    }
    pub async fn get_filetype(&self) -> FileType {
        let t = self.content_type.lock().await;
        match *t {
            ContentType::File => FileType::RegularFile,
            ContentType::Directory(_) => FileType::Directory,
        }
    }
    pub fn get_parent(&self) -> u64 {
        self.parent
    }
    pub async fn get_stat(&self) -> ReplyEntry {
        match self.get_tyep().await {
            ContentType::File => default_file_entry(self.inode),
            ContentType::Directory(_) => default_dic_entry(self.inode),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct FileTreeEntry {
    tree_items: Vec<Item>,
    #[serde(default)]
    total_count: u64,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct ApiData {
    #[serde(default)]
    file_tree: std::collections::HashMap<String, FileTreeEntry>,
    #[serde(default)]
    tree_items: Vec<Item>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct ApiResponse {
    req_result: bool,
    data: ApiData,
    err_message: String,
}

impl ApiResponse {
    /// Get all items from tree_items in data
    #[allow(dead_code)]
    fn get_items(&self) -> Vec<Item> {
        self.data.tree_items.clone()
    }
}

impl Iterator for ApiResponse {
    type Item = Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.data.tree_items.pop()
    }
}

#[allow(dead_code)]
struct ApiResponseExt {
    _req_result: bool,
    data: Vec<ItemExt>,
    _err_message: String,
}

#[derive(Debug)]
pub struct DictionaryError {
    pub message: String,
}

impl std::fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DictionaryError: {}", self.message)
    }
}

impl std::error::Error for DictionaryError {}
impl From<reqwest::Error> for DictionaryError {
    fn from(err: reqwest::Error) -> Self {
        DictionaryError {
            message: err.to_string(),
        }
    }
}

// Get Mega dictionary tree from server
#[allow(unused)]
async fn fetch_tree(path: &str) -> Result<ApiResponse, DictionaryError> {
    static CLIENT: Lazy<Client> = Lazy::new(|| {
        Client::builder()
            .timeout(Duration::from_secs(10)) // 10 second timeout for network requests
            .build()
            .unwrap_or_else(|_| Client::new()) // Fallback to default client if builder fails
    });
    let client = CLIENT.clone();
    // Remove leading slash from path to avoid double slashes in URL
    let clean_path = path.trim_start_matches('/');
    let url = format!("{}/api/v1/tree?path=/{}", config::base_url(), clean_path);
    let kk = client.get(&url).send().await;
    if kk.is_err() {
        return Err(DictionaryError {
            message: "Failed to fetch tree".to_string(),
        });
    }
    let resp: Result<ApiResponse, reqwest::Error> = kk.unwrap().json().await;

    match resp {
        Ok(resp) => Ok(resp),
        Err(e) => Err(e.into()),
    }
}

fn reqwest_err_to_io(err: reqwest::Error) -> io::Error {
    if err.is_timeout() {
        io::Error::new(io::ErrorKind::TimedOut, err.to_string())
    } else {
        io::Error::other(err.to_string())
    }
}

/// Download a file from the server using its OID/hash with retry mechanism.
///
/// IMPORTANT: This returns an error on failures. Callers must NOT treat failures as empty files,
/// otherwise we may poison persistent caches with 0-byte content.
async fn fetch_file(oid: &str) -> io::Result<Vec<u8>> {
    let start = Instant::now();
    let file_blob_endpoint = config::file_blob_endpoint();
    let url = format!("{file_blob_endpoint}/{oid}");
    static CLIENT: Lazy<Client> = Lazy::new(|| {
        Client::builder()
            .timeout(Duration::from_secs(30)) // 30 second timeout for file downloads (files may be large)
            .build()
            .unwrap_or_else(|_| Client::new()) // Fallback to default client if builder fails
    });
    let client = CLIENT.clone();

    const MAX_RETRIES: u32 = 3;
    // Base delay for linear backoff: 100ms, 200ms, 300ms for attempts 0, 1, 2
    // Linear backoff is appropriate here since we only retry a few times with short delays
    const RETRY_DELAY_MS: u64 = 100;

    // Retry logic for network errors
    for attempt in 0..MAX_RETRIES {
        // Send GET request
        let response = match client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                if attempt < MAX_RETRIES - 1 {
                    // Retry on network errors (timeout, connection refused, etc.)
                    debug!(
                        "Failed to fetch file with OID: {oid} (attempt {}/{}), retrying...",
                        attempt + 1,
                        MAX_RETRIES
                    );
                    debug!("  URL: {url}");
                    debug!("  Error: {e}");
                    tokio::time::sleep(Duration::from_millis(
                        RETRY_DELAY_MS * (attempt + 1) as u64,
                    ))
                    .await;
                    continue;
                } else {
                    // Final attempt failed
                    debug!(
                        "Failed to fetch file with OID: {oid} after {} attempts",
                        MAX_RETRIES
                    );
                    debug!("  URL: {url}");
                    debug!("  Error: {e}");
                    return Err(reqwest_err_to_io(e));
                }
            }
        };

        // Ensure that the response status is successful
        if response.status().is_success() {
            // Get the binary data from the response body
            match response.bytes().await {
                Ok(bytes) => {
                    debug!(
                        "fetch_file: ok oid={} bytes={} elapsed={:.2}s",
                        oid,
                        bytes.len(),
                        start.elapsed().as_secs_f64()
                    );
                    return Ok(bytes.to_vec());
                }
                Err(e) => {
                    if attempt < MAX_RETRIES - 1 {
                        debug!(
                            "Failed to read content for OID: {oid} (attempt {}/{}), retrying...",
                            attempt + 1,
                            MAX_RETRIES
                        );
                        debug!("  URL: {url}");
                        debug!("  Error: {e}");
                        tokio::time::sleep(Duration::from_millis(
                            RETRY_DELAY_MS * (attempt + 1) as u64,
                        ))
                        .await;
                        continue;
                    } else {
                        debug!(
                            "Failed to read content for OID: {oid} after {} attempts",
                            MAX_RETRIES
                        );
                        debug!("  URL: {url}");
                        debug!("  Error: {e}");
                        return Err(reqwest_err_to_io(e));
                    }
                }
            }
        } else {
            let status = response.status();
            debug!("Failed to fetch file: HTTP {} for OID: {oid}", status);
            debug!("  URL: {url}");
            let kind = if status == reqwest::StatusCode::NOT_FOUND {
                io::ErrorKind::NotFound
            } else {
                io::ErrorKind::Other
            };
            return Err(io::Error::new(
                kind,
                format!("HTTP {}: failed to fetch oid {}", status, oid),
            ));
        }
    }
    Err(io::Error::other(format!("failed to fetch oid {oid}")))
}

/// Fetch file size (in bytes) for a blob without downloading the full content.
///
/// Strategy:
/// 1) Try HTTP HEAD and read Content-Length.
/// 2) Fallback to GET with Range: bytes=0-0 and parse Content-Range.
#[cfg(test)]
static FETCH_FILE_SIZE_CALLS: AtomicUsize = AtomicUsize::new(0);
async fn fetch_file_size(oid: &str) -> Option<u64> {
    #[cfg(test)]
    FETCH_FILE_SIZE_CALLS.fetch_add(1, Ordering::Relaxed);

    use reqwest::header::{CONTENT_LENGTH, CONTENT_RANGE, RANGE};

    let file_blob_endpoint = config::file_blob_endpoint();
    let url = format!("{file_blob_endpoint}/{oid}");
    static CLIENT: Lazy<Client> = Lazy::new(|| {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new())
    });
    let client = CLIENT.clone();

    // 1) HEAD
    if let Ok(resp) = client.head(&url).send().await {
        if resp.status().is_success() {
            if let Some(v) = resp.headers().get(CONTENT_LENGTH) {
                if let Ok(s) = v.to_str() {
                    if let Ok(n) = s.parse::<u64>() {
                        debug!("fetch_file_size: head ok oid={} size={}", oid, n);
                        return Some(n);
                    }
                }
            }
        }
    }

    // 2) Range GET (0-0)
    let resp = client
        .get(&url)
        .header(RANGE, "bytes=0-0")
        .send()
        .await
        .ok()?;
    if !(resp.status().is_success() || resp.status().as_u16() == 206) {
        return None;
    }

    // Prefer Content-Range: bytes 0-0/12345
    if let Some(v) = resp.headers().get(CONTENT_RANGE) {
        if let Ok(s) = v.to_str() {
            if let Some(total) = s.rsplit('/').next() {
                if let Ok(n) = total.parse::<u64>() {
                    debug!("fetch_file_size: range ok oid={} size={}", oid, n);
                    return Some(n);
                }
            }
        }
    }

    // Fallback: Content-Length on a 206 should be 1, but if server ignores Range it may be full.
    if let Some(v) = resp.headers().get(CONTENT_LENGTH) {
        if let Ok(s) = v.to_str() {
            if let Ok(n) = s.parse::<u64>() {
                // Only accept plausible small values if Range was honored.
                if n <= 1 {
                    return Some(n);
                }
            }
        }
    }

    None
}

async fn fetch_dir(path: &str) -> Result<ApiResponseExt, DictionaryError> {
    let start = Instant::now();
    static CLIENT: Lazy<Client> = Lazy::new(|| {
        Client::builder()
            .timeout(Duration::from_secs(10)) // 10 second timeout for network requests
            .build()
            .unwrap_or_else(|_| Client::new()) // Fallback to default client if builder fails
    });
    let client = CLIENT.clone();

    let clean_path = path.trim_start_matches('/');
    let url = format!(
        "{}/api/v1/tree/content-hash?path=/{}",
        config::base_url(),
        clean_path
    );

    const MAX_RETRIES: u32 = 3;
    // Base delay for linear backoff: 100ms, 200ms, 300ms for attempts 0, 1, 2
    // Linear backoff is appropriate here since we only retry a few times with short delays
    const RETRY_DELAY_MS: u64 = 100;

    // Retry logic for network errors
    for attempt in 0..MAX_RETRIES {
        let response = match client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                if attempt < MAX_RETRIES - 1 {
                    // Retry on network errors (timeout, connection refused, etc.)
                    debug!(
                        "Failed to fetch tree: {e} (attempt {}/{}), retrying...",
                        attempt + 1,
                        MAX_RETRIES
                    );
                    debug!("  URL: {url}");
                    debug!("  Path: {path}");
                    tokio::time::sleep(Duration::from_millis(
                        RETRY_DELAY_MS * (attempt + 1) as u64,
                    ))
                    .await;
                    continue;
                } else {
                    // Final attempt failed
                    debug!("Failed to fetch tree: {e} after {} attempts", MAX_RETRIES);
                    debug!("  URL: {url}");
                    debug!("  Path: {path}");
                    return Ok(ApiResponseExt {
                        _req_result: false,
                        data: Vec::new(),
                        _err_message: format!("Failed to fetch tree: {e}"),
                    });
                }
            }
        };

        // Check response status before parsing JSON
        if !response.status().is_success() {
            let status = response.status();
            // Don't retry on HTTP errors (4xx, 5xx) - these are permanent failures
            if status.is_client_error() || status.is_server_error() {
                debug!("Failed to fetch tree: HTTP {} for path: {path}", status);
                debug!("  URL: {url}");
                return Ok(ApiResponseExt {
                    _req_result: false,
                    data: Vec::new(),
                    _err_message: format!("HTTP {}: Failed to fetch tree for path: {path}", status),
                });
            }
            // For other status codes, retry
            if attempt < MAX_RETRIES - 1 {
                debug!(
                    "Unexpected HTTP status {} for path: {path} (attempt {}/{}), retrying...",
                    status,
                    attempt + 1,
                    MAX_RETRIES
                );
                tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * (attempt + 1) as u64))
                    .await;
                continue;
            }
        }

        // Parse JSON response
        let tree_info: TreeInfoResponse = match response.json().await {
            Ok(info) => info,
            Err(e) => {
                if attempt < MAX_RETRIES - 1 {
                    debug!(
                        "Failed to parse commit info: {e} (attempt {}/{}), retrying...",
                        attempt + 1,
                        MAX_RETRIES
                    );
                    tokio::time::sleep(Duration::from_millis(
                        RETRY_DELAY_MS * (attempt + 1) as u64,
                    ))
                    .await;
                    continue;
                } else {
                    debug!(
                        "Failed to parse commit info: {e} after {} attempts",
                        MAX_RETRIES
                    );
                    return Ok(ApiResponseExt {
                        _req_result: false,
                        data: Vec::new(),
                        _err_message: format!("Failed to parse commit info: {e}"),
                    });
                }
            }
        };

        if !tree_info.req_result {
            debug!(
                "server response fetch dir error: {:?}",
                tree_info.err_message
            );
            return Ok(ApiResponseExt {
                _req_result: false,
                data: Vec::new(),
                _err_message: format!(
                    "server response fetch dir error: {:?}",
                    tree_info.err_message
                ),
            });
        }

        // Successfully parsed response, process data
        let mut data = Vec::with_capacity(tree_info.data.len());

        let base_path = if path.is_empty() || path == "/" {
            "".to_string()
        } else if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{path}/")
        };

        for info in tree_info.data {
            let full_path = if base_path.is_empty() {
                format!("/{}", info.name)
            } else {
                format!("/{}{}", base_path.trim_start_matches('/'), info.name)
            };

            data.push(ItemExt {
                item: Item {
                    name: info.name,
                    path: full_path,
                    content_type: info.content_type,
                },
                hash: info.oid,
            });
        }

        debug!(
            "fetch_dir: ok user_path={:?} items={} elapsed={:.2}s",
            path,
            data.len(),
            start.elapsed().as_secs_f64()
        );
        return Ok(ApiResponseExt {
            _req_result: true,
            data,
            _err_message: String::new(),
        });
    }

    // All retries exhausted
    Ok(ApiResponseExt {
        _req_result: false,
        data: Vec::new(),
        _err_message: format!("Failed to fetch tree after {} attempts", MAX_RETRIES),
    })
}

/// Get the directory hash from the server
async fn fetch_get_dir_hash(path: &str) -> Result<ApiResponseExt, DictionaryError> {
    let start = Instant::now();
    static CLIENT: Lazy<Client> = Lazy::new(|| {
        Client::builder()
            .timeout(Duration::from_secs(10)) // 10 second timeout for network requests
            .build()
            .unwrap_or_else(|_| Client::new()) // Fallback to default client if builder fails
    });
    let client = CLIENT.clone();

    let clean_path = path.trim_start_matches('/');
    let url = format!(
        "{}/api/v1/tree/dir-hash?path=/{}",
        config::base_url(),
        clean_path
    );

    let response = match client.get(&url).send().await {
        Ok(resp) => resp,
        Err(_) => {
            return Err(DictionaryError {
                message: "Failed to fetch tree".to_string(),
            });
        }
    };

    let tree_info: TreeInfoResponse = match response.json().await {
        Ok(info) => info,
        Err(e) => {
            return Err(DictionaryError {
                message: format!("Failed to parse commit info: {e}"),
            });
        }
    };

    if !tree_info.req_result {
        return Err(DictionaryError {
            message: tree_info.err_message,
        });
    }

    let mut data = Vec::with_capacity(tree_info.data.len());

    let base_path = if path.is_empty() || path == "/" {
        "".to_string()
    } else if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{path}/")
    };

    for info in tree_info.data {
        let full_path = if base_path.is_empty() {
            format!("/{}", info.name)
        } else {
            format!("/{}{}", base_path.trim_start_matches('/'), info.name)
        };

        data.push(ItemExt {
            item: Item {
                name: info.name,
                path: full_path,
                content_type: info.content_type,
            },
            hash: info.oid,
        });
    }

    debug!(
        "fetch_get_dir_hash: ok user_path={:?} items={} elapsed={:.2}s",
        path,
        data.len(),
        start.elapsed().as_secs_f64()
    );
    Ok(ApiResponseExt {
        _req_result: true,
        data,
        _err_message: String::new(),
    })
}

/// Represents a directory with its metadata
/// - hash: represents the hash of the last commit that modified this directory
/// - file_list: represents the list of files and subdirectories in this directory, with boolean values indicating if they still exist
pub struct DirItem {
    hash: String,
    file_list: HashMap<String, bool>,
    /// Whether we have fetched this directory's children listing at least once.
    ///
    /// This distinguishes "not loaded yet" from a legitimately empty directory.
    loaded: bool,
    /// Best-effort in-memory TTL to avoid frequent remote refreshes (used by load_dir/watch paths).
    last_sync: Option<Instant>,
}

pub struct DictionaryStore {
    inodes: Arc<Mutex<HashMap<u64, Arc<DicItem>>>>,
    // dirs: Arc<Mutex<HashMap<String, DirItem>>>, //save all the dirs.
    dirs: Arc<DashMap<String, DirItem>>, // save all the dirs.
    /// Per-directory async locks to avoid concurrent loads producing duplicate inodes.
    dir_locks: Arc<DashMap<String, Arc<Mutex<()>>>>,
    next_inode: AtomicU64,
    radix_trie: Arc<Mutex<radix_trie::Trie<String, u64>>>,
    persistent_path_store: Arc<TreeStorage>, // persistent path store for saving and retrieving file paths
    max_depth: Arc<usize>,                   // max depth for loading directories
    pub init_notify: Arc<Notify>,            // used in dir_test to notify the start of the test..
    /// One-way readiness latch for consumers (e.g., Antares) that need root inode available.
    ready: AtomicBool,
    /// Guards `import_arc` so we don't start multiple background imports concurrently for the same store.
    import_started: AtomicBool,
    persistent_content_store: Arc<ContentStorage>, // persistent content store for saving and retrieving file contents
    persistent_size_store: Arc<SizeStorage>,       // persistent size store (inode -> size)
    open_buff: Arc<DashMap<u64, Vec<u8>>>,         // buffer for open files
    /// Tracks executable bit for files. Populated when downloading git blobs.
    exec_flags: Arc<DashMap<u64, bool>>,
    /// Base path for subdirectory mounting (e.g., "/third-party/mega").
    /// When set, only content under this path is accessible.
    base_path: String,
    /// Root directory for this store's on-disk DB files (path.db/content.db/size.db/markers).
    ///
    /// Important for Antares/base_path mounts where multiple Dicfuse instances must not share
    /// the same sled DB directory.
    store_dir: String,
    /// Metadata stat mode (fast avoids any network size probing).
    stat_mode: config::DicfuseStatMode,
    /// TTL for directory refresh paths (load_dir/watch).
    dir_sync_ttl: Duration,
    /// Best-effort memory bound for open file cache.
    open_buff_max_bytes: u64,
    open_buff_max_files: usize,
    open_buff_bytes: AtomicU64,
}

#[allow(unused)]
impl DictionaryStore {
    pub async fn new() -> Self {
        let tree_store = TreeStorage::new().expect("Failed to create TreeStorage");
        let store_dir = config::store_path().to_string();
        DictionaryStore {
            next_inode: AtomicU64::new(1),
            inodes: Arc::new(Mutex::new(HashMap::new())),
            radix_trie: Arc::new(Mutex::new(radix_trie::Trie::new())),
            persistent_path_store: Arc::new(tree_store),
            dirs: Arc::new(DashMap::new()),
            dir_locks: Arc::new(DashMap::new()),
            max_depth: Arc::new(config::load_dir_depth()),
            init_notify: Arc::new(Notify::new()),
            ready: AtomicBool::new(false),
            import_started: AtomicBool::new(false),
            persistent_content_store: Arc::new(
                ContentStorage::new().expect("Failed to create ContentStorage"),
            ),
            persistent_size_store: Arc::new(
                SizeStorage::new().expect("Failed to create SizeStorage"),
            ),
            open_buff: Arc::new(DashMap::new()),
            exec_flags: Arc::new(DashMap::new()),
            base_path: String::new(),
            store_dir,
            stat_mode: config::dicfuse_stat_mode(),
            dir_sync_ttl: Duration::from_secs(config::dicfuse_dir_sync_ttl_secs()),
            open_buff_max_bytes: config::dicfuse_open_buff_max_bytes(),
            open_buff_max_files: config::dicfuse_open_buff_max_files(),
            open_buff_bytes: AtomicU64::new(0),
        }
    }

    pub async fn new_with_store_path(store_path: &str) -> Self {
        let tree_store =
            TreeStorage::new_with_path(store_path).expect("Failed to create TreeStorage");
        let store_dir = store_path.to_string();
        DictionaryStore {
            next_inode: AtomicU64::new(1),
            inodes: Arc::new(Mutex::new(HashMap::new())),
            radix_trie: Arc::new(Mutex::new(radix_trie::Trie::new())),
            persistent_path_store: Arc::new(tree_store),
            dirs: Arc::new(DashMap::new()),
            dir_locks: Arc::new(DashMap::new()),
            max_depth: Arc::new(config::load_dir_depth()),
            init_notify: Arc::new(Notify::new()),
            ready: AtomicBool::new(false),
            import_started: AtomicBool::new(false),
            persistent_content_store: Arc::new(
                ContentStorage::new_with_path(store_path).expect("Failed to create ContentStorage"),
            ),
            persistent_size_store: Arc::new(
                SizeStorage::new_with_path(store_path).expect("Failed to create SizeStorage"),
            ),
            open_buff: Arc::new(DashMap::new()),
            exec_flags: Arc::new(DashMap::new()),
            base_path: String::new(),
            store_dir,
            stat_mode: config::dicfuse_stat_mode(),
            dir_sync_ttl: Duration::from_secs(config::dicfuse_dir_sync_ttl_secs()),
            open_buff_max_bytes: config::dicfuse_open_buff_max_bytes(),
            open_buff_max_files: config::dicfuse_open_buff_max_files(),
            open_buff_bytes: AtomicU64::new(0),
        }
    }

    /// Create a new DictionaryStore with a base path and an explicit store path.
    ///
    /// This is primarily used to avoid DB lock conflicts when multiple Dicfuse instances
    /// (e.g., different Antares mounts) are created concurrently. Each instance can use a
    /// dedicated `store_path` directory to keep its sled DB files isolated.
    pub async fn new_with_base_path_and_store_path(base_path: &str, store_path: &str) -> Self {
        let tree_store =
            TreeStorage::new_with_path(store_path).expect("Failed to create TreeStorage");
        let store_dir = store_path.to_string();
        let is_subdir_mount = !(base_path.is_empty() || base_path == "/");
        let max_depth = if is_subdir_mount {
            config::antares_load_dir_depth()
        } else {
            config::load_dir_depth()
        };
        let stat_mode = if is_subdir_mount {
            config::antares_dicfuse_stat_mode()
        } else {
            config::dicfuse_stat_mode()
        };
        let dir_sync_ttl = if is_subdir_mount {
            Duration::from_secs(config::antares_dicfuse_dir_sync_ttl_secs())
        } else {
            Duration::from_secs(config::dicfuse_dir_sync_ttl_secs())
        };
        let open_buff_max_bytes = if is_subdir_mount {
            config::antares_dicfuse_open_buff_max_bytes()
        } else {
            config::dicfuse_open_buff_max_bytes()
        };
        let open_buff_max_files = if is_subdir_mount {
            config::antares_dicfuse_open_buff_max_files()
        } else {
            config::dicfuse_open_buff_max_files()
        };
        DictionaryStore {
            next_inode: AtomicU64::new(1),
            inodes: Arc::new(Mutex::new(HashMap::new())),
            radix_trie: Arc::new(Mutex::new(radix_trie::Trie::new())),
            persistent_path_store: Arc::new(tree_store),
            dirs: Arc::new(DashMap::new()),
            dir_locks: Arc::new(DashMap::new()),
            max_depth: Arc::new(max_depth),
            init_notify: Arc::new(Notify::new()),
            ready: AtomicBool::new(false),
            import_started: AtomicBool::new(false),
            persistent_content_store: Arc::new(
                ContentStorage::new_with_path(store_path).expect("Failed to create ContentStorage"),
            ),
            persistent_size_store: Arc::new(
                SizeStorage::new_with_path(store_path).expect("Failed to create SizeStorage"),
            ),
            open_buff: Arc::new(DashMap::new()),
            exec_flags: Arc::new(DashMap::new()),
            base_path: base_path.to_string(),
            store_dir,
            stat_mode,
            dir_sync_ttl,
            open_buff_max_bytes,
            open_buff_max_files,
            open_buff_bytes: AtomicU64::new(0),
        }
    }

    /// Returns true if this call is the first one to start import for this store.
    pub fn try_start_import(&self) -> bool {
        !self.import_started.swap(true, Ordering::AcqRel)
    }

    /// Create a new DictionaryStore with a base path for subdirectory mounting.
    ///
    /// When `base_path` is set (e.g., "/third-party/mega"), the store will:
    /// - Only load content under the specified path
    /// - Remap paths so the base_path becomes the root "/"
    ///
    /// # Arguments
    /// * `base_path` - The subdirectory path to use as root (e.g., "/third-party/mega")
    pub async fn new_with_base_path(base_path: &str) -> Self {
        // IMPORTANT: avoid opening the same sled DB path multiple times (can cause lock conflicts).
        // We isolate per-base_path stores under a deterministic subdirectory of the configured store_path.
        let store_root = config::store_path();
        let store_path =
            super::compute_store_dir_for_base_path_with_store_root(store_root, base_path);

        std::fs::create_dir_all(&store_path)
            .expect("Failed to create per-base_path store directory");

        Self::new_with_base_path_and_store_path(base_path, &store_path).await
    }

    /// Get the base path for this store.
    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// Convert a user-visible path to the real path in the monorepo.
    ///
    /// When base_path = "/third-party/mega":
    /// - "/" -> "/third-party/mega"
    /// - "/src" -> "/third-party/mega/src"
    pub fn to_real_path(&self, user_path: &str) -> String {
        if self.base_path.is_empty() || self.base_path == "/" {
            user_path.to_string()
        } else {
            let base = self.base_path.trim_end_matches('/');
            if user_path == "/" || user_path.is_empty() {
                base.to_string()
            } else {
                format!("{}{}", base, user_path)
            }
        }
    }

    /// Convert a real monorepo path to user-visible path.
    ///
    /// When base_path = "/third-party/mega":
    /// - "/third-party/mega" -> "/"
    /// - "/third-party/mega/src" -> "/src"
    ///
    /// Returns None if the path is not under the base_path.
    pub fn to_user_path(&self, real_path: &str) -> Option<String> {
        if self.base_path.is_empty() || self.base_path == "/" {
            Some(real_path.to_string())
        } else {
            let base = self.base_path.trim_end_matches('/');
            if real_path == base {
                Some("/".to_string())
            } else if real_path.starts_with(&format!("{}/", base)) {
                Some(real_path[base.len()..].to_string())
            } else {
                None
            }
        }
    }

    #[inline(always)]
    pub fn max_depth(&self) -> usize {
        *self.max_depth
    }

    pub fn stat_mode(&self) -> config::DicfuseStatMode {
        self.stat_mode
    }

    pub fn dir_sync_ttl(&self) -> Duration {
        self.dir_sync_ttl
    }

    fn open_buff_cache_enabled(&self) -> bool {
        self.open_buff_max_bytes > 0 && self.open_buff_max_files > 0
    }

    fn open_buff_maybe_evict_for_insert(&self, new_len: u64) {
        if !self.open_buff_cache_enabled() {
            return;
        }

        let cur_files = self.open_buff.len();
        let cur_bytes = self.open_buff_bytes.load(Ordering::Acquire);
        if cur_files >= self.open_buff_max_files
            || cur_bytes.saturating_add(new_len) > self.open_buff_max_bytes
        {
            debug!(
                "dicfuse: open_buff eviction: cur_files={} cur_bytes={} new_len={} max_files={} max_bytes={}",
                cur_files,
                cur_bytes,
                new_len,
                self.open_buff_max_files,
                self.open_buff_max_bytes
            );
            self.open_buff.clear();
            self.open_buff_bytes.store(0, Ordering::Release);
        }
    }

    fn open_buff_insert(&self, inode: u64, content: Vec<u8>) {
        if !self.open_buff_cache_enabled() {
            return;
        }

        let new_len = content.len() as u64;
        if new_len > self.open_buff_max_bytes {
            // Too large to cache under current limits; keep only persisted content.
            return;
        }

        self.open_buff_maybe_evict_for_insert(new_len);
        if let Some(old) = self.open_buff.insert(inode, content) {
            self.open_buff_bytes
                .fetch_sub(old.len() as u64, Ordering::AcqRel);
        }
        self.open_buff_bytes.fetch_add(new_len, Ordering::AcqRel);
    }

    fn open_buff_remove(&self, inode: u64) {
        if let Some((_, old)) = self.open_buff.remove(&inode) {
            self.open_buff_bytes
                .fetch_sub(old.len() as u64, Ordering::AcqRel);
        }
    }

    fn mark_ready(&self) {
        // Mark as ready first, then notify.
        self.ready.store(true, Ordering::Release);
        // Wake current waiters.
        self.init_notify.notify_waiters();
        // Also store a permit for any future waiter to avoid missed notifications.
        self.init_notify.notify_one();
    }

    pub async fn wait_for_ready(&self) {
        // Wait for the store to be initialized. This is a latch-style wait:
        // - If already ready, return immediately.
        // - Otherwise, wait for a notification (with a stored permit via notify_one).
        while !self.ready.load(Ordering::Acquire) {
            self.init_notify.notified().await;
        }
    }

    fn dir_lock_for_path(&self, user_path: &str) -> Arc<Mutex<()>> {
        self.dir_locks
            .entry(user_path.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    fn inode_to_user_path(&self, inode: u64) -> io::Result<String> {
        if inode == 1 {
            return Ok("/".to_string());
        }
        let p = self.persistent_path_store.get_all_path(inode)?;
        Ok(format!("/{}", p.to_string()))
    }

    async fn upsert_inode(&self, parent: u64, item: ItemExt) -> io::Result<u64> {
        // Use radix trie as the authoritative mapping from path -> inode.
        let key = GPath::from(item.item.path.clone()).to_string();
        let existing = {
            let trie = self.radix_trie.lock().await;
            trie.get(&key).copied()
        };

        if let Some(inode) = existing {
            return Ok(inode);
        }

        self.update_inode(parent, item).await
    }

    /// Ensure a directory's children are loaded at least once.
    ///
    /// This enables "lazy directory loading": if a path lookup reaches an unloaded directory,
    /// we fetch one directory listing from the server, populate inode/path mappings, and then
    /// subsequent lookups become pure-local.
    pub async fn ensure_dir_loaded(&self, parent_inode: u64) -> io::Result<()> {
        let parent_user_path = self.inode_to_user_path(parent_inode)?;
        ensure_dir_tracked(&self.dirs, &parent_user_path);

        // Fast path: already loaded.
        if let Some(dir) = self.dirs.get(&parent_user_path) {
            if dir.loaded {
                return Ok(());
            }
        }

        let lock = self.dir_lock_for_path(&parent_user_path);
        let _guard = lock.lock().await;

        // Re-check under lock.
        if let Some(dir) = self.dirs.get(&parent_user_path) {
            if dir.loaded {
                return Ok(());
            }
        }

        // Fetch remote listing and populate children.
        let real_parent_path = self.to_real_path(&parent_user_path);
        let fetched = fetch_dir(&real_parent_path)
            .await
            .map_err(|e| io::Error::other(e.to_string()))?;
        if !fetched._req_result {
            return Err(io::Error::other(fetched._err_message));
        }

        let items: Vec<ItemExt> = fetched
            .data
            .into_iter()
            .filter_map(|it| map_itemext_to_user(self, it))
            .collect();

        // Track entries seen during this sync using file_list booleans (same pattern as load_dir()).
        if let Some(mut dir) = self.dirs.get_mut(&parent_user_path) {
            for it in &items {
                dir.file_list.insert(it.item.path.clone(), true);
            }
        }

        // Populate tree/trie for children.
        for it in items {
            let is_dir = it.item.is_dir();
            let child_path = it.item.path.clone(); // USER path (leading '/')

            let child_inode = self.upsert_inode(parent_inode, it.clone()).await?;

            if let Ok(existing) = self.persistent_path_store.get_item(child_inode) {
                if existing.hash != it.hash {
                    let _ = self
                        .persistent_path_store
                        .update_item_hash(child_inode, it.hash.clone());
                    // If a file changed, invalidate cached content so reads refetch lazily.
                    if !is_dir {
                        let _ = self.remove_file_by_node(child_inode);
                    }
                }
            }

            if is_dir {
                ensure_dir_tracked(&self.dirs, &child_path);
                if let Some(mut child_dir) = self.dirs.get_mut(&child_path) {
                    child_dir.hash = it.hash;
                }
            }
        }

        // Prune missing children and finalize loaded state.
        let mut remove_items = Vec::new();
        if let Some(mut dir) = self.dirs.get_mut(&parent_user_path) {
            dir.file_list.retain(|path, seen| {
                if !*seen {
                    remove_items.push(path.clone());
                    false
                } else {
                    *seen = false;
                    true
                }
            });
            dir.loaded = true;
            dir.last_sync = Some(Instant::now());
        }

        for path in remove_items {
            if let Ok(inode) = self.get_inode_from_path(&path).await {
                let _ = self.persistent_path_store.remove_item(inode);
                let _ = self.remove_file_by_node(inode);
            }
        }

        Ok(())
    }
    async fn update_inode(&self, parent: u64, item: ItemExt) -> std::io::Result<u64> {
        let alloc_inode = self
            .next_inode
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        assert!(alloc_inode < READONLY_INODE);

        let prw = self.persistent_path_store.clone();
        if let Ok(pinode) = prw.get_item(parent) {
            // insert info to a radix_trie for path match.
            self.radix_trie
                .lock()
                .await
                .insert(GPath::from(item.item.path.clone()).to_string(), alloc_inode);
            prw.insert_item(alloc_inode, parent, item);
            //prw.append_child(parent, alloc_inode);
        } else {
            //error...
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Parent inode not found",
            ));
        }

        Ok(alloc_inode)
    }

    #[async_recursion]
    async fn traverse_directory(
        &self,
        current_path: &str,
        base_path: &str,
        depth_items: &mut HashMap<i32, BTreeSet<String>>,
    ) -> Result<(), io::Error> {
        let current_inode = match self.get_inode_from_path(current_path).await {
            Ok(inode) => inode,
            Err(_) => return Ok(()),
        };

        let current_item = self.persistent_path_store.get_item(current_inode)?;

        if !current_item.is_dir() {
            return Ok(());
        }

        let children = current_item.get_children();

        for child_inode in children {
            let child_item = self.persistent_path_store.get_item(child_inode)?;
            let child_name = child_item.get_name();

            let child_full_path = if current_path == "/" {
                format!("/{child_name}")
            } else {
                format!("{current_path}/{child_name}")
            };

            let relative_path = if base_path == "/" {
                child_full_path
                    .strip_prefix('/')
                    .unwrap_or(&child_full_path)
                    .to_string()
            } else if child_full_path == base_path {
                ".".to_string()
            } else if child_full_path.starts_with(&format!("{base_path}/")) {
                child_full_path[base_path.len() + 1..].to_string()
            } else {
                continue;
            };

            let depth = if relative_path == "." {
                0
            } else {
                relative_path.chars().filter(|&c| c == '/').count() as i32
            };

            depth_items.entry(depth).or_default().insert(relative_path);

            if child_item.is_dir() {
                self.traverse_directory(&child_full_path, base_path, depth_items)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn add_temp_point(&self, path: &str) -> Result<u64, io::Error> {
        let item_path = path.to_string();
        let mut path = GPath::from(path.to_string());
        let name = path.pop();
        let parent = self.get_by_path(&path.to_string()).await?;
        let name = match name {
            Some(n) => n,
            None => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid path")),
        };
        self.update_inode(
            parent.get_inode(),
            ItemExt {
                item: Item {
                    name,
                    path: item_path,
                    content_type: INODE_DICTIONARY.to_string(),
                },
                hash: String::new(),
            },
        )
        .await
    }

    /// Recursively traverses and returns all files and directories under the specified base directory, grouped by depth relative to base_dir
    /// Returns HashMap<depth_level, relative_paths_set> where depth 0 = direct children, depth 1 = grandchildren, etc.
    pub async fn get_dir_by_path(&self, base_dir: &str) -> HashMap<i32, BTreeSet<String>> {
        let mut depth_items: HashMap<i32, BTreeSet<String>> = HashMap::new();

        let normalized_base_dir = if base_dir.is_empty() || base_dir == "." {
            "/".to_string()
        } else if !base_dir.starts_with('/') {
            format!("/{base_dir}")
        } else {
            base_dir.to_string()
        };

        if (self.get_inode_from_path(&normalized_base_dir).await).is_ok() {
            if let Err(e) = self
                .traverse_directory(&normalized_base_dir, &normalized_base_dir, &mut depth_items)
                .await
            {
                warn!("Error traversing directory {normalized_base_dir}: {e}");
            }

            if normalized_base_dir != "/" {
                depth_items.entry(0).or_default();
            }
        } else {
            debug!("Base directory {normalized_base_dir} not found");
        }

        depth_items
    }

    /// When a file changed,the parent directory's hash changed too.
    /// So we need to update the ancestors' hash .
    pub async fn update_ancestors_hash(&self, inode: u64) {
        // Walk up from `inode` to root, refreshing directory hashes.
        //
        // NOTE: the directory tree should be a DAG rooted at inode=1. In case of corrupted
        // on-disk state (e.g., bad parent pointers), guard against cycles to avoid infinite loops.
        const MAX_STEPS: usize = 1024;
        let mut visited: std::collections::HashSet<u64> = std::collections::HashSet::new();

        let mut cur = inode;
        for _ in 0..MAX_STEPS {
            if !visited.insert(cur) {
                warn!("update_ancestors_hash: detected parent cycle at inode {cur}; aborting");
                return;
            }
            let cur_item = match self.persistent_path_store.get_item(cur) {
                Ok(i) => i,
                Err(e) => {
                    warn!("update_ancestors_hash: missing inode {cur}: {e}");
                    return;
                }
            };
            let parent_inode = cur_item.get_parent();
            if parent_inode == 0 || parent_inode == 1 {
                return;
            }

            let user_path = "/".to_string()
                + &self
                    .persistent_path_store
                    .get_all_path(parent_inode)
                    .unwrap_or_else(|_| GPath::new())
                    .to_string();
            let real_path = self.to_real_path(&user_path);

            let hash = get_dir_hash(&real_path).await;
            if hash.is_empty() {
                return;
            }

            if let Err(e) = self
                .persistent_path_store
                .update_item_hash(parent_inode, hash.to_owned())
            {
                warn!("update_ancestors_hash: failed to update hash for inode {parent_inode}: {e}");
                return;
            }

            ensure_dir_tracked(&self.dirs, &user_path);
            if let Some(mut dir) = self.dirs.get_mut(&user_path) {
                dir.hash = hash;
            }

            cur = parent_inode;
        }

        warn!(
            "update_ancestors_hash: exceeded MAX_STEPS={MAX_STEPS} starting from inode {inode}; aborting"
        );
        // TODO(dicfuse): Increment a metric/counter here so operators can detect corrupted
        // parent pointers / excessive depth issues in production.
    }
    /// When scorpio start,if the db is not empty, we need to load all the files to the memory.
    #[async_recursion]
    /// Loads directories recursively from the parent path into memory.
    async fn load_dirs(&self, path: PathBuf, parent_inode: u64) -> Result<(), io::Error> {
        let root_item = self.persistent_path_store.get_item(parent_inode)?;
        let children = root_item.get_children();
        // If the directory exists in our persisted path store, treat it as "loaded".
        // Even for empty directories, this avoids re-fetching on first access after restart.
        let loaded = true;
        self.dirs.insert(
            path.to_string_lossy().to_string(),
            DirItem {
                hash: root_item.hash.to_owned(),
                file_list: HashMap::new(),
                loaded,
                last_sync: Some(Instant::now()),
            },
        );
        for child in children {
            self.next_inode.fetch_max(child, Ordering::Relaxed);
            let child_item = self.persistent_path_store.get_item(child)?;
            let child_path = path.join(child_item.get_name());
            self.dirs
                .get_mut(&path.to_string_lossy().to_string())
                .unwrap()
                .file_list
                .insert(child_path.to_string_lossy().to_string(), false);
            self.radix_trie.lock().await.insert(
                GPath::from(child_path.to_string_lossy().to_string()).to_string(),
                child,
            );

            if child_item.is_dir() {
                self.load_dirs(child_path, child).await?;
            }
        }
        Ok(())
    }

    /// When the scorpio start,we need to load the directories and files from db to the memory.
    pub async fn load_db(&self) -> Result<(), io::Error> {
        let mut path = PathBuf::from("/");
        self.load_dirs(path, 1).await?;
        Ok(())
    }

    pub async fn import(&self) {
        let items = fetch_dir("").await.unwrap().data;

        //let root_inode = self.inodes.lock().await.get(&1).unwrap().clone();
        // deque for bus.
        let mut queue = VecDeque::<u64>::new();
        self.update_inode(
            0,
            ItemExt {
                item: Item {
                    name: String::new(),
                    path: String::new(),
                    content_type: INODE_DICTIONARY.to_string(),
                },
                hash: String::new().to_string(),
            },
        )
        .await
        .unwrap();
        for it in items {
            let is_dir = it.item.content_type == INODE_DICTIONARY;
            let it_inode = self.update_inode(1, it).await.unwrap();
            if is_dir {
                queue.push_back(it_inode);
            }
        }

        loop {
            //BFS to look up all dictionary
            if queue.is_empty() {
                break;
            }
            let one_inode = queue.pop_front().unwrap();
            let mut new_items = Vec::new();

            let it = self.persistent_path_store.get_all_path(one_inode).unwrap();
            let path = it.to_string();
            debug!("fetch path :{path}");
            // get tree by parent inode.
            new_items = fetch_dir(&path).await.unwrap().data;

            // Insert all new inode.
            for newit in new_items {
                //println!("import item :{:?}",newit);
                let is_dir = newit.item.is_dir();
                let new_inode = self.update_inode(one_inode, newit).await.unwrap(); // Await the update_inode call
                                                                                    // push to queue to BFS.
                if is_dir {
                    queue.push_back(new_inode);
                }
            }
        }
        //queue.clear();
    }

    pub async fn find_path(&self, inode: u64) -> Option<GPath> {
        self.persistent_path_store.get_all_path(inode).ok()
    }
    pub async fn get_inode(&self, inode: u64) -> Result<StorageItem, io::Error> {
        self.persistent_path_store.get_item(inode)
    }

    pub async fn get_by_path(&self, path: &str) -> Result<StorageItem, io::Error> {
        let inode = if path.is_empty() || path == "/" {
            1
        } else {
            let binding = self.radix_trie.lock().await;
            *binding
                .get(path)
                .ok_or(io::Error::new(io::ErrorKind::NotFound, "path not found"))?
        };

        self.get_inode(inode).await
    }
    /// Get the inode from path
    pub async fn get_inode_from_path(&self, path: &str) -> Result<u64, io::Error> {
        let inode = if path.is_empty() || path == "/" {
            1
        } else {
            let binding = self.radix_trie.lock().await;
            *binding
                .get(&GPath::from(path.to_owned()).to_string())
                .ok_or(io::Error::new(io::ErrorKind::NotFound, "path not found"))?
        };

        Ok(inode)
    }

    pub async fn do_readdir(
        &self,
        parent: u64,
        fh: u64,
        offset: u64,
    ) -> Result<Vec<StorageItem>, io::Error> {
        //  1. get the parent directory.
        let item = self.get_inode(parent).await?; // current_dictionary
        let mut parent_path = self.find_path(parent).await.unwrap();
        parent_path.pop();

        let parent_item = self.get_by_path(&parent_path.to_string()).await?;

        let mut re = vec![item.clone(), parent_item.clone()];

        // 2. make sure this item is a directory
        if item.is_dir() {
            // 3. Get the children of the directory

            let children = self.persistent_path_store.get_children(parent)?;
            // 4. build a list of StorageItem structs for each child.
            for child in children.iter() {
                re.push(child.clone());
            }
            Ok(re)
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "Not a directory"))
        }
    }
}

/// File operations interface for in-memory file management
/// Provides functions to handle file content stored in memory buffer (open_buff)
impl DictionaryStore {
    pub fn set_executable(&self, inode: u64, executable: bool) {
        self.exec_flags.insert(inode, executable);
    }

    pub fn is_executable(&self, inode: u64) -> bool {
        self.exec_flags
            .get(&inode)
            .map(|v| *v.value())
            .unwrap_or(false)
    }

    pub fn get_file_len(&self, inode: u64) -> u64 {
        self.open_buff.get(&inode).map_or(0, |v| v.len() as u64)
    }

    pub fn get_persisted_size(&self, inode: u64) -> Option<u64> {
        self.persistent_size_store.get_size(inode).ok().flatten()
    }

    pub fn set_persisted_size(&self, inode: u64, size: u64) {
        let _ = self.persistent_size_store.set_size(inode, size);
    }

    fn get_open_buff_len(&self, inode: u64) -> Option<u64> {
        self.open_buff.get(&inode).map(|v| v.len() as u64)
    }

    /// Get a file size suitable for `stat`:
    /// - Prefer persisted size (from size.db)
    /// - If content is already cached in memory, use that (and persist it)
    /// - As a last resort, fetch size from remote by hash/oid (HEAD/Range) and persist it
    pub async fn get_or_fetch_file_size(&self, inode: u64, oid: &str) -> u64 {
        if let Some(persisted) = self.get_persisted_size(inode) {
            // NOTE: 0 is a valid cached size (empty file). We treat "not cached" as None.
            return persisted;
        }

        // Fast-path for known-empty blobs (git empty blob hash).
        if !oid.is_empty() && oid == EMPTY_BLOB_OID {
            self.set_persisted_size(inode, 0);
            return 0;
        }

        if let Some(mem_len) = self.get_open_buff_len(inode) {
            // Only trust in-memory length when it's non-zero. A zero-length buffer may be:
            // - a legitimate empty file, or
            // - a previously poisoned cache (e.g., fetch failure incorrectly cached as empty).
            // For len==0, fall through to remote size discovery.
            if mem_len > 0 {
                self.set_persisted_size(inode, mem_len);
                return mem_len;
            }
        }

        // If file content exists on disk (content.db), load it into memory and use its length.
        if let Ok(content) = self.persistent_content_store.get_file_content(inode) {
            let len = content.len() as u64;
            if len > 0 {
                self.set_persisted_size(inode, len);
                return len;
            }
        }

        if oid.is_empty() {
            return 0;
        }

        if let Some(sz) = fetch_file_size(oid).await {
            self.set_persisted_size(inode, sz);
            return sz;
        }

        0
    }

    /// Get a file size suitable for `stat`, honoring this store's `stat_mode`.
    ///
    /// - `Fast`: never probes remote size; returns persisted size if present, otherwise 0.
    /// - `Accurate`: may probe remote size (HEAD/Range) when missing.
    pub async fn file_size_for_stat(&self, inode: u64, oid: &str) -> u64 {
        match self.stat_mode() {
            config::DicfuseStatMode::Fast => {
                if !oid.is_empty() && oid == EMPTY_BLOB_OID {
                    self.set_persisted_size(inode, 0);
                    return 0;
                }
                if let Some(persisted) = self.get_persisted_size(inode) {
                    return persisted;
                }
                if let Some(mem_len) = self.get_open_buff_len(inode) {
                    if mem_len > 0 {
                        self.set_persisted_size(inode, mem_len);
                        return mem_len;
                    }
                }
                0
            }
            config::DicfuseStatMode::Accurate => self.get_or_fetch_file_size(inode, oid).await,
        }
    }
    pub fn remove_file_by_node(&self, inode: u64) -> Result<(), io::Error> {
        // Best-effort: clear size metadata too, so concurrent getattr during refetch cannot
        // observe a stale persisted size (especially problematic if it was cached as 0).
        if let Err(e) = self.persistent_size_store.remove_size(inode) {
            warn!(
                "remove_file_by_node: failed to remove persisted size for inode {}: {}",
                inode, e
            );
        }
        self.persistent_content_store.remove_file(inode)?;
        self.open_buff_remove(inode);
        Ok(())
    }
    /// Save to db and then save in the memory.
    pub fn save_file(&self, inode: u64, content: Vec<u8>) {
        // Persist size metadata so getattr can report correct size even with lazy content.
        let _ = self
            .persistent_size_store
            .set_size(inode, content.len() as u64);
        self.persistent_content_store
            .insert_file(inode, &content)
            .expect("Failed to save file content");
        self.open_buff_insert(inode, content);
    }
    /// Check if the file exists in the memory.
    pub fn file_exists(&self, inode: u64) -> bool {
        if self.open_buff.contains_key(&inode) {
            return true;
        }
        // Prefer size.db as an existence check to avoid reading full blob bytes from sled.
        if let Ok(Some(_)) = self.persistent_size_store.get_size(inode) {
            return true;
        }
        // Backward-compat: older caches may have content without size metadata.
        self.persistent_content_store
            .get_file_content(inode)
            .is_ok()
    }
    /// Get the file content from the memory.
    pub fn get_file_content(&self, inode: u64) -> Option<Ref<'_, u64, Vec<u8>>> {
        self.open_buff.get(&inode)
    }

    /// Get the file content from persistent storage (content.db).
    pub fn get_persisted_file_content(&self, inode: u64) -> io::Result<Vec<u8>> {
        self.persistent_content_store.get_file_content(inode)
    }

    /// Download the file content from the server and save it to the db and memory.
    pub async fn fetch_file_content(&self, inode: u64, oid: &str) -> io::Result<()> {
        let content = fetch_file(oid).await?;
        self.save_file(inode, content);
        Ok(())
    }

    /// Best-effort: ensure a file's content is loaded into memory (`open_buff`) if it's persisted.
    ///
    /// Returns:
    /// - Ok(true)  if content is now available in memory
    /// - Ok(false) if content is not in persistent storage
    /// - Err(_)    on storage errors
    pub fn ensure_file_loaded(&self, inode: u64) -> io::Result<bool> {
        if self.open_buff.contains_key(&inode) {
            return Ok(true);
        }
        match self.persistent_content_store.get_file_content(inode) {
            Ok(content) => {
                // Best-effort cache: obey open_buff bounds.
                let can_cache = self.open_buff_cache_enabled()
                    && (content.len() as u64) <= self.open_buff_max_bytes;
                if can_cache {
                    self.open_buff_insert(inode, content);
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }
    /// Return the content of a file by its path.
    pub async fn get_file_content_by_path(&self, path: &str) -> Result<Vec<u8>, io::Error> {
        let inode = self.get_inode_from_path(path).await?;
        if let Some(content) = self.get_file_content(inode) {
            return Ok(content.to_vec());
        }
        self.persistent_content_store.get_file_content(inode)
    }
}

/// Convert an `ItemExt` whose `item.path` is a real monorepo path into a user-visible path,
/// according to the store's `base_path` remapping rules.
///
/// - For the default/root view (`base_path == ""` or "/"), this is a no-op.
/// - For subdirectory mounts (`base_path == "/third-party/mega"`), this strips the base prefix:
///   - real: "/third-party/mega/scorpio/Cargo.toml" -> user: "/scorpio/Cargo.toml"
fn map_itemext_to_user(store: &DictionaryStore, it: ItemExt) -> Option<ItemExt> {
    // When base_path is set, the server should only return items under base_path.
    // If it returns out-of-scope paths, we drop them. Log (rate-limited) to aid debugging.
    static DROPPED_OUT_OF_SCOPE: std::sync::atomic::AtomicUsize =
        std::sync::atomic::AtomicUsize::new(0);

    let ItemExt {
        item: Item {
            name,
            path,
            content_type,
        },
        hash,
    } = it;
    let user_path = match store.to_user_path(&path) {
        Some(p) => p,
        None => {
            if !store.base_path.is_empty() && store.base_path != "/" {
                let n = DROPPED_OUT_OF_SCOPE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if n < 20 {
                    debug!(
                        "map_itemext_to_user: dropping out-of-scope item path={:?} base_path={:?}",
                        path, store.base_path
                    );
                }
            }
            return None;
        }
    };
    Some(ItemExt {
        item: Item {
            name,
            path: user_path,
            content_type,
        },
        hash,
    })
}

/// Ensure a directory path is tracked in `dirs` so callers can safely update `file_list`.
fn ensure_dir_tracked(dirs: &DashMap<String, DirItem>, path: &str) {
    if !dirs.contains_key(path) {
        dirs.insert(
            path.to_string(),
            DirItem {
                hash: String::new(),
                file_list: HashMap::new(),
                loaded: false,
                last_sync: None,
            },
        );
    }
}

fn import_done_marker_path(store: &DictionaryStore) -> PathBuf {
    PathBuf::from(&store.store_dir).join(".dicfuse_import_done")
}

async fn reset_store_for_import(store: &DictionaryStore) {
    // Clear persisted DBs (path + content) to avoid duplicating inodes for existing paths.
    // This is necessary because `update_inode()` currently always allocates a fresh inode.
    let _ = store.persistent_path_store.clear_all();
    let _ = store.persistent_content_store.clear_all();
    let _ = store.persistent_size_store.clear_all();

    // Clear in-memory caches.
    store.open_buff.clear();
    store.open_buff_bytes.store(0, Ordering::Release);
    store.exec_flags.clear();
    store.dirs.clear();
    store.dir_locks.clear();
    store.inodes.lock().await.clear();
    *store.radix_trie.lock().await = radix_trie::Trie::new();

    store.next_inode.store(1, Ordering::Relaxed);
    store.ready.store(false, Ordering::Release);
}

/// Loads subdirectories from a remote server into an empty parent directory up to a specified depth.
///
/// # Arguments
/// * `parent_path` - The path to an empty directory where subdirectories will be loaded.
/// * `max_depth` - The maximum absolute depth of subdirectories to load, relative to the root.
///
/// # TODO(dicfuse-antares-integration)
/// - Implement parallel file content prefetching for common build artifacts
/// - Add configurable rate limiting per remote server
/// - Support resumable loading for network interruptions
/// - Add metrics/tracing for load time analysis
pub async fn load_dir_depth(store: Arc<DictionaryStore>, parent_path: String, max_depth: usize) {
    let start_time = std::time::Instant::now();
    // IMPORTANT: `parent_path` is the USER-visible path (i.e., relative to the mount root).
    // For subdirectory mounts, we must translate it to the real monorepo path for network calls.
    let parent_path = if parent_path.is_empty() {
        "/".to_string()
    } else {
        parent_path
    };
    let real_parent_path = store.to_real_path(&parent_path);

    info!(
        "[load_dir_depth] starting load (user={parent_path:?} real={real_parent_path:?} max_depth={max_depth})"
    );

    // Ensure we don't concurrently load the same directory (e.g., import_arc + on-demand lookups).
    let dir_lock = store.dir_lock_for_path(&parent_path);
    let _dir_guard = dir_lock.lock().await;

    let queue = Arc::new(SegQueue::new());
    let fetched = match fetch_dir(&real_parent_path).await {
        Ok(r) => r,
        Err(e) => {
            warn!(
                "[load_dir_depth] Failed to fetch directory listing for real={real_parent_path:?}: {e}"
            );
            return;
        }
    };
    if !fetched._req_result {
        warn!(
            "[load_dir_depth] Server reported failure for real={real_parent_path:?}: {}",
            fetched._err_message
        );
        return;
    }

    // Convert all returned paths into user-visible paths (base_path-stripped for subdir mounts).
    let items: Vec<ItemExt> = fetched
        .data
        .into_iter()
        .filter_map(|it| map_itemext_to_user(store.as_ref(), it))
        .collect();
    info!(
        "[load_dir_depth] fetched {} items (user={parent_path:?} real={real_parent_path:?})",
        items.len()
    );
    // only count the directories.
    let dir_count = items.iter().filter(|it| it.item.is_dir()).count();
    let file_count = items.len() - dir_count;
    info!(
        "[load_dir_depth] discovered {} dirs and {} files (user={parent_path:?})",
        dir_count, file_count
    );
    let active_producers = Arc::new(AtomicUsize::new(dir_count));
    // let active_producers = Arc::new(AtomicUsize::new(items.len()));
    {
        let locks = store.dirs.clone();
        // Ensure the parent directory is tracked in the in-memory `dirs` map.
        // This prevents panics in update paths (watch_dir/update_dir) and allows us to
        // safely record child entries in `file_list`.
        ensure_dir_tracked(&locks, &parent_path);
        if let Some(mut parent_dir) = locks.get_mut(&parent_path) {
            parent_dir.loaded = true;
            parent_dir.last_sync = Some(Instant::now());
        }

        // Get parent inode once before the loop (with proper error handling)
        let parent_node = match store.get_inode_from_path(&parent_path).await {
            Ok(inode) => inode,
            Err(e) => {
                warn!(
                    "[load_dir_depth] parent_path not found in radix_trie (user={parent_path:?}): {e}"
                );
                return;
            }
        };

        for it in items {
            let is_dir = it.item.is_dir();
            let path = it.item.path.to_owned();

            // Update parent's file_list for change tracking.
            if let Some(mut parent_dir) = locks.get_mut(&parent_path) {
                parent_dir.file_list.insert(path.to_owned(), false);
            }

            let it_inode = match store.upsert_inode(parent_node, it.clone()).await {
                Ok(inode) => inode,
                Err(e) => {
                    warn!("[load_dir_depth] upsert_inode failed (path={path:?}): {e}");
                    continue;
                }
            };

            if is_dir {
                queue.push(it_inode);
                locks.insert(
                    path,
                    DirItem {
                        hash: it.hash,
                        file_list: HashMap::new(),
                        loaded: false,
                        last_sync: None,
                    },
                );
            } else {
                // NOTE: Do NOT prefetch file contents during directory tree loading.
                // Dicfuse should fetch file contents on-demand on read() to keep initial load fast,
                // especially for large monorepos.
            }
        }
    }

    // Release the parent directory lock before spawning workers for deeper traversal.
    drop(_dir_guard);

    let worker_count = std::cmp::max(1, config::fetch_file_thread());
    let mut workers = Vec::with_capacity(worker_count);

    // clone shared resource.
    let queue = Arc::clone(&queue);
    let persistent_path_store = store.persistent_path_store.clone();

    // Init mulity work thraed
    for _ in 0..worker_count {
        let queue = Arc::clone(&queue);
        let path_store = persistent_path_store.clone();
        let store = store.clone();
        let producers = Arc::clone(&active_producers);

        workers.push(tokio::spawn(async move {
            // Rate limiting: add small delay between requests to avoid overwhelming the server
            const REQUEST_DELAY_MS: u64 = 10; // 10ms delay between requests per worker

            while producers.load(Ordering::Acquire) > 0 || !queue.is_empty() {
                if let Some(inode) = queue.pop() {
                    // Build USER path from inode (e.g., "/scorpio/src").
                    let path = match path_store.get_all_path(inode) {
                        Ok(p) => "/".to_string() + &p.to_string(),
                        Err(e) => {
                            debug!(
                                "[load_dir_depth] Worker failed to resolve inode {} to path: {e}",
                                inode
                            );
                            producers.fetch_sub(1, Ordering::Release);
                            continue;
                        }
                    };
                    // Translate to REAL monorepo path for network calls (handles base_path mounts).
                    let real_path = store.to_real_path(&path);
                    let remaining_producers = producers.load(Ordering::Acquire);
                    let queue_size = queue.len();
                    if queue_size.is_multiple_of(10) || remaining_producers.is_multiple_of(50) {
                        debug!(
                            "[load_dir_depth] processing user path={path} real={real_path} remaining_producers={} queue_size={}",
                            remaining_producers,
                            queue_size
                        );
                    }

                    // Prevent concurrent loads of the same directory (e.g., on-demand lookup + import).
                    let dir_lock = store.dir_lock_for_path(&path);
                    let _dir_guard = dir_lock.lock().await;

                    // Rate limiting: small delay before each request to avoid overwhelming server
                    tokio::time::sleep(Duration::from_millis(REQUEST_DELAY_MS)).await;

                    // get all children inode
                    let result = fetch_dir(&real_path).await;
                    match result {
                        Ok(resp) => {
                            if !resp._req_result {
                                debug!(
                                    "[load_dir_depth] fetch_dir failed for real={real_path:?}: {}",
                                    resp._err_message
                                );
                            } else {
                                // Convert to USER-visible paths for storage.
                                let new_items: Vec<ItemExt> = resp
                                    .data
                                    .into_iter()
                                    .filter_map(|it| map_itemext_to_user(store.as_ref(), it))
                                    .collect();

                                // Ensure parent dir exists before updating file_list.
                                ensure_dir_tracked(&store.dirs, &path);
                                if let Some(mut parent_dir) = store.dirs.get_mut(&path) {
                                    parent_dir.loaded = true;
                                    parent_dir.last_sync = Some(Instant::now());
                                }

                                for newit in new_items {
                                    let is_dir = newit.item.is_dir();
                                    let tmp_path = newit.item.path.to_owned(); // USER path

                                    // Track child in parent's file_list.
                                    if let Some(mut parent_dir) = store.dirs.get_mut(&path) {
                                        parent_dir.file_list.insert(tmp_path.to_owned(), false);
                                    }

                                    let new_inode =
                                        match store.upsert_inode(inode, newit.clone()).await {
                                        Ok(i) => i,
                                        Err(e) => {
                                            debug!(
                                                "[load_dir_depth] update_inode failed for parent={} child={}: {e}",
                                                inode, tmp_path
                                            );
                                            continue;
                                        }
                                    };

                                    if is_dir {
                                        // If it's a directory, push it to the queue and add the producer count.
                                        if tmp_path.matches('/').count() < max_depth {
                                            producers.fetch_add(1, Ordering::Relaxed);
                                            queue.push(new_inode);
                                        } else {
                                            // Depth limit reached; do not enqueue deeper traversal.
                                        }
                                        store.dirs.insert(
                                            tmp_path,
                                            DirItem {
                                                hash: newit.hash,
                                                file_list: HashMap::new(),
                                                loaded: false,
                                                last_sync: None,
                                            },
                                        );
                                    } else {
                                        // NOTE: Do NOT prefetch file contents during directory tree loading.
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // Log error but continue - still need to decrement producer count
                            debug!("Failed to fetch directory real={real_path:?} (user={path:?}): {e}");
                        }
                    };

                    // Always decrement producer count after processing, regardless of success or failure
                    producers.fetch_sub(1, Ordering::Release);
                } else {
                    // If there are no active producers and the queue is empty, exit the loop
                    let current_producers = producers.load(Ordering::Acquire);
                    if current_producers == 0 {
                        return;
                    }
                    // yield to wait unfinished tasks
                    // Add a small delay to avoid busy waiting
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        }));
    }

    // wait for all workers to complete
    // while let Some(worker) = workers.pop() {
    //     worker.await.expect("Worker panicked");
    // }
    info!("[load_dir_depth] waiting for {} workers", worker_count);
    debug!(
        "[load_dir_depth] state: producers={} queue_size={}",
        active_producers.load(Ordering::Acquire),
        queue.len()
    );
    join_all(workers).await;
    let elapsed = start_time.elapsed();
    info!(
        "[load_dir_depth] completed loading directory tree from user={parent_path:?} in {:.2}s",
        elapsed.as_secs_f64()
    );
}

pub async fn import_arc(store: Arc<DictionaryStore>) {
    // Dicfuse always exposes a USER-visible root "/".
    // If `base_path` is configured, USER paths are remapped to REAL monorepo paths:
    //   user "/scorpio" -> real "/third-party/mega/scorpio"
    let user_root = "/".to_string();
    let real_root = store.to_real_path(&user_root);
    let marker_path = import_done_marker_path(store.as_ref());

    // 1) Try to load existing DB state.
    // The import marker acts as a durable latch that the DB directory is initialized for this store.
    if store.load_db().await.is_ok() {
        let marker_ok = marker_path.exists();
        let has_root = store.persistent_path_store.get_item(1).is_ok();

        // Always make sure the root is tracked in dirs.
        ensure_dir_tracked(&store.dirs, &user_root);

        if marker_ok && has_root {
            store.mark_ready();
            if store.max_depth() > 0 {
                let watch_path = user_root.clone();
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                        watch_dir_path(store.clone(), &watch_path).await;
                    }
                });
            }
            return;
        }

        warn!(
            "[import_arc] Existing DB is not usable (has_root={has_root}, marker_ok={marker_ok}); rebuilding (real_root={real_root:?}, base_path={:?})",
            store.base_path
        );
        // fall through to rebuild (after clearing state)
    }

    // Clear any partially-initialized store before rebuilding to avoid inode duplication.
    reset_store_for_import(store.as_ref()).await;

    // 2) Initialize root inode (idempotent: overwrite is fine).
    let _ = store.persistent_path_store.insert_item(
        1,
        UNKNOW_INODE,
        ItemExt {
            item: Item {
                name: "".to_string(),
                path: user_root.clone(),
                content_type: INODE_DICTIONARY.to_string(),
            },
            hash: String::new(),
        },
    );
    let root_item = DicItem {
        inode: 1,
        // Keep root consistent with other GPath usage (no leading slash, empty segments removed).
        path_name: GPath::new(),
        content_type: Arc::new(Mutex::new(ContentType::Directory(false))),
        children: Mutex::new(HashMap::new()),
        parent: UNKNOW_INODE, // root has no parent
    };
    store.inodes.lock().await.insert(1, root_item.into());
    ensure_dir_tracked(&store.dirs, &user_root);

    // Mark ready as soon as the root inode exists so Antares can mount immediately.
    // Directory entries will be populated lazily on lookup/readdir, while import continues.
    store.mark_ready();

    // Mark store initialization as complete so subsequent startups can trust the on-disk cache.
    if let Some(parent) = marker_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&marker_path, b"ok\n");

    // Limit concurrent warmups/imports across stores to avoid remote pressure spikes.
    let _permit = global_import_semaphore()
        .acquire_owned()
        .await
        .expect("global import semaphore closed");

    // Always do a shallow root listing once (best-effort). This warms the root and persists children,
    // while still keeping mount time-to-usable low (root is already ready).
    if let Err(e) = store.ensure_dir_loaded(1).await {
        warn!(
            "[import_arc] ensure_dir_loaded(root) failed (user_root={user_root:?} real_root={real_root:?}): {e}"
        );
    }

    // Optional deep prewarm: disabled by default for Antares subdir mounts (max_depth=0).
    if store.max_depth() > 0 {
        let max_depth = store.max_depth() + 2;
        info!(
            "[import_arc] Prewarming directory tree (user_root={user_root:?} real_root={real_root:?} max_depth={max_depth} load_dir_depth={})",
            store.max_depth()
        );
        load_dir_depth(store.clone(), user_root.clone(), max_depth).await;
        info!("[import_arc] Prewarm completed (user_root={user_root:?} real_root={real_root:?})");
    } else {
        info!(
            "[import_arc] Skipping deep prewarm (max_depth=0) for base_path={:?}",
            store.base_path
        );
    }

    store.mark_ready();

    // Spawn background task for periodic directory watching.
    // For Antares subdir mounts (default max_depth=0), we skip the watcher to avoid background
    // remote storms; directories are refreshed lazily when accessed.
    if store.max_depth() > 0 {
        let watch_path = user_root;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                watch_dir_path(store.clone(), &watch_path).await;
            }
        });
    }
}

/// Get the directory hash from the server
async fn get_dir_hash(path: &str) -> String {
    let data = fetch_get_dir_hash(path).await.unwrap().data;
    // no need to filter by name, just return the first item.the server ensure the name is unique.
    if data.len() == 1 {
        data[0].hash.to_owned()
    } else {
        String::new()
    }
}

#[async_recursion]
/// Preloads a directory and its subdirectories up to a specified depth from a remote server.
///
/// The function fetches the directory's hash to verify its existence. If the directory is empty,
/// it loads subdirectories up to `max_depth` (absolute depth relative to the root directory).
/// If non-empty, it compares the hash to detect changes: if unchanged, it processes the local
/// directory; if changed, it fetches and loads the updated directory from the remote server.
///
/// # Arguments
/// * `parent_path` - The path to the directory to preload (must be a valid, existing path).
/// * `max_depth` - The maximum absolute depth of subdirectories to load, relative to the root.
pub async fn load_dir(
    store: Arc<DictionaryStore>,
    parent_path: String,
    max_depth: usize,
) -> Result<bool, io::Error> {
    // `parent_path` is USER-visible (relative to mount root). Normalize it.
    let parent_path = if parent_path.is_empty() {
        "/".to_string()
    } else if !parent_path.starts_with('/') {
        format!("/{parent_path}")
    } else {
        parent_path
    };

    if parent_path.matches('/').count() >= max_depth {
        info!("max depth reached for path: {parent_path}");
        return Ok(false);
    }
    if max_depth < store.max_depth() + 2 {
        info!("max depth is less than config, skipping: {parent_path}");
        return Ok(false);
    }

    // Resolve inode and ensure the path is a valid directory.
    let parent_inode = match store.get_inode_from_path(&parent_path).await {
        Ok(inode) => inode,
        Err(e) => {
            warn!("load_dir: invalid path (not found): {parent_path}, err: {e}");
            return Err(io::Error::new(io::ErrorKind::NotFound, e));
        }
    };

    let tree_db = store.persistent_path_store.clone();
    let dirs = store.dirs.clone();

    // Check underlying storage item type.
    let parent_item = match tree_db.get_item(parent_inode) {
        Ok(item) => item,
        Err(e) => {
            warn!("load_dir: failed to get item for {parent_path}: {e}");
            return Err(io::Error::other(e));
        }
    };
    if !parent_item.is_dir() {
        warn!("load_dir: path is not a directory: {parent_path}");
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path is not a directory",
        ));
    }

    // Ensure we are tracking this directory in the in-memory dirs map.
    ensure_dir_tracked(&dirs, &parent_path);

    // Best-effort TTL: avoid repeated refreshes of the same directory in hot loops.
    let ttl = store.dir_sync_ttl();
    if ttl != Duration::from_secs(0) {
        if let Some(d) = dirs.get(&parent_path) {
            if let Some(ts) = d.last_sync {
                if ts.elapsed() < ttl {
                    debug!(
                        "load_dir: skip refresh due to TTL (user={parent_path:?} ttl={:?})",
                        ttl
                    );
                    return Ok(false);
                }
            }
        }
    }

    // Translate USER path -> REAL path for network calls.
    let real_parent_path = store.to_real_path(&parent_path);

    let self_hash = get_dir_hash(&real_parent_path).await;

    //the dir may be deleted.
    if self_hash.is_empty() {
        // This can happen if the directory does not exist (yet) or the server is temporarily
        // unavailable. Keep it at debug to avoid log spam under high concurrency.
        debug!("Directory {real_parent_path} is empty or not found, no items to load.");
        return Ok(true);
    }
    info!("load_dir parent_path user={parent_path:?} real={real_parent_path:?}");

    // If this directory was never loaded, do a shallow one-level load and stop.
    // This avoids treating "not loaded" as "empty" and prevents deep/BFS prewarm on hot paths.
    let (was_loaded, file_list_empty) = dirs
        .get(&parent_path)
        .map(|d| (d.loaded, d.file_list.is_empty()))
        .unwrap_or((false, true));
    if !was_loaded {
        store.ensure_dir_loaded(parent_inode).await?;
        if let Some(mut dir) = dirs.get_mut(&parent_path) {
            dir.loaded = true;
            dir.hash = self_hash.to_owned();
            dir.last_sync = Some(Instant::now());
        }
        let _ = tree_db.update_item_hash(parent_inode, self_hash);
        return Ok(true);
    }

    // For already-loaded but empty directories, just refresh the hash and return.
    if file_list_empty {
        let mut changed = false;
        if let Some(mut dir) = dirs.get_mut(&parent_path) {
            if dir.hash != self_hash {
                dir.hash = self_hash.to_owned();
                changed = true;
            }
            dir.last_sync = Some(Instant::now());
        }
        if changed {
            let _ = tree_db.update_item_hash(parent_inode, self_hash);
        }
        return Ok(changed);
    }
    // if the dir's hash is same as the parent dir's hash,
    //then check the subdir from the db,no need to get from the server..
    let cached_hash = dirs
        .get(&parent_path)
        .map(|d| d.hash.clone())
        .unwrap_or_default();
    if cached_hash == self_hash {
        if let Some(mut dir) = dirs.get_mut(&parent_path) {
            dir.last_sync = Some(Instant::now());
        }
        let item = match store.persistent_path_store.get_item(parent_inode) {
            Ok(i) => i,
            Err(e) => {
                warn!("load_dir: failed to get parent inode {parent_inode}: {e}");
                return Ok(false);
            }
        };
        for child in item.get_children() {
            let child_item = match store.persistent_path_store.get_item(child) {
                Ok(i) => i,
                Err(_) => continue,
            };
            if child_item.is_dir() {
                let child_user = match tree_db.get_all_path(child) {
                    Ok(p) => "/".to_string() + &p.to_string(),
                    Err(_) => continue,
                };
                if let Err(e) = load_dir(store.clone(), child_user, max_depth).await {
                    warn!("load_dir: failed for child dir {child}: {e}");
                }
            }
        }
        return Ok(false);
    }
    //last, if the dir's hash is different from the parent dir's hash,
    //then fetch the dir from the server.
    let fetched = fetch_dir(&real_parent_path)
        .await
        .map_err(|e| io::Error::other(e.to_string()))?;
    if !fetched._req_result {
        warn!(
            "load_dir: fetch_dir failed for real={real_parent_path:?}: {}",
            fetched._err_message
        );
        return Ok(false);
    }
    let items: Vec<ItemExt> = fetched
        .data
        .into_iter()
        .filter_map(|it| map_itemext_to_user(store.as_ref(), it))
        .collect();

    if let Some(mut dir) = dirs.get_mut(&parent_path) {
        dir.hash = self_hash.to_owned();
        dir.last_sync = Some(Instant::now());
    }
    tree_db
        .update_item_hash(parent_inode, self_hash)
        .map_err(io::Error::other)?;
    for it in items {
        let is_dir = it.item.is_dir();
        let path = it.item.path.to_owned(); // USER path

        // the item already exists in the parent directory.
        let existed = dirs
            .get(&parent_path)
            .map(|d| d.file_list.contains_key(&path))
            .unwrap_or(false);
        if existed {
            if let Some(mut dir) = dirs.get_mut(&parent_path) {
                dir.file_list.insert(path.to_owned(), true);
            }
            if is_dir {
                info!("hash changes dir {path:?}");
                if let Err(e) = load_dir(store.clone(), path.to_owned(), max_depth).await {
                    warn!("load_dir failed for updated dir {path:?}: {e}");
                }
            } else if let Ok(inode) = store.get_inode_from_path(&path).await {
                if let Ok(item) = store.persistent_path_store.get_item(inode) {
                    if item.hash != it.hash {
                        let _ = tree_db.update_item_hash(inode, it.hash.to_owned());
                        // Invalidate any cached content; it will be fetched lazily on read().
                        let _ = store.remove_file_by_node(inode);
                    }
                }
            }
        } else {
            if let Some(mut dir) = dirs.get_mut(&parent_path) {
                dir.file_list.insert(path.to_owned(), true);
            }
            info!("load dir add new file {path:?}");
            let _new_node = match store.upsert_inode(parent_inode, it.clone()).await {
                Ok(i) => i,
                Err(e) => {
                    warn!("load_dir: update_inode failed for {path:?}: {e}");
                    continue;
                }
            };
            //fetch a new dir.
            if is_dir {
                info!("add dir {path:?}");
                dirs.insert(
                    path.to_owned(),
                    DirItem {
                        hash: it.hash,
                        file_list: HashMap::new(),
                        loaded: false,
                        last_sync: None,
                    },
                );
                // Do not deep-prewarm here; allow on-demand lazy loads and/or background watchers.
            } else {
                // Do not prefetch content for newly discovered files; fetch lazily on read().
            }
        }
    }
    let mut remove_items = Vec::new();
    if let Some(mut dir) = dirs.get_mut(&parent_path) {
        dir.file_list.retain(|path, v| {
            let result = *v;
            if !(*v) {
                remove_items.push(path.clone());
            } else {
                *v = false;
            }
            result
        });
        dir.last_sync = Some(Instant::now());
    }
    for item in remove_items {
        if let Ok(inode) = store.get_inode_from_path(&item).await {
            info!("delete {inode:?} {item} ");
            let _ = tree_db.remove_item(inode);
            let _ = store.remove_file_by_node(inode);
        }
    }
    Ok(true)
}

#[async_recursion]
/// This function is only used to update the directory which has been loaded.
/// It will update the directory but do not load the new directory.
pub async fn update_dir(store: Arc<DictionaryStore>, parent_path: String) {
    // Keep the watcher logic simple and resilient: delegate to `load_dir`, which already:
    // - Converts USER paths to REAL paths for network calls (base_path-aware)
    // - Updates in-memory `dirs` and on-disk tree state
    // - Avoids panics on missing entries
    let parent_path = if parent_path.is_empty() {
        "/".to_string()
    } else if !parent_path.starts_with('/') {
        format!("/{parent_path}")
    } else {
        parent_path
    };
    let max_depth = store.max_depth() + 2 + parent_path.matches('/').count();
    if let Err(e) = load_dir(store, parent_path, max_depth).await {
        warn!("update_dir: load_dir failed: {e}");
    }
}

/// Watch the directory and update the dictionary has loaded.
pub async fn watch_dir(store: Arc<DictionaryStore>) {
    update_dir(store, "/".to_string()).await;
}

/// Watch and update a specific directory path (for subdirectory mounting support)
pub async fn watch_dir_path(store: Arc<DictionaryStore>, path: &str) {
    update_dir(store, path.to_string()).await;
}

/// Test-only helper methods for DictionaryStore
#[cfg(test)]
impl DictionaryStore {
    /// Insert a mock item for testing purposes.
    /// This allows tests to set up the internal state without network calls.
    pub async fn insert_mock_item(&self, inode: u64, parent: u64, name: &str, is_dir: bool) {
        // Build a deterministic full path based on the parent entry.
        // Note: DictionaryStore internally stores paths without leading "/" in the radix trie
        // (via `GPath::from(...).to_string()`), but the Item path is represented with leading "/".
        let full_path = if inode == 1 || name.is_empty() {
            "/".to_string()
        } else {
            let parent_path = self
                .persistent_path_store
                .get_all_path(parent)
                .map(|p| p.to_string())
                .unwrap_or_default();
            if parent_path.is_empty() {
                format!("/{name}")
            } else {
                format!("/{parent_path}/{name}")
            }
        };

        let item = ItemExt {
            item: Item {
                name: name.to_string(),
                path: full_path.clone(),
                content_type: if is_dir {
                    INODE_DICTIONARY.to_string()
                } else {
                    INODE_FILE.to_string()
                },
            },
            hash: String::new(),
        };
        let _ = self.persistent_path_store.insert_item(inode, parent, item);

        // Keep trie in sync so `get_by_path` works in tests.
        self.radix_trie
            .lock()
            .await
            .insert(GPath::from(full_path).to_string(), inode);
    }
}

#[cfg(test)]
mod tests {
    use radix_trie::TrieCommon;

    use super::*;
    #[tokio::test]
    #[ignore]
    async fn test_fetch_tree_success() {
        let path: &str = "/third-party/mega";

        let _result = fetch_tree(path).await.unwrap();
    }

    #[test]
    fn test_tree() {
        let mut t = radix_trie::Trie::<String, u64>::new();
        t.insert(String::from("/a"), 0);
        t.insert(String::from("/a/b"), 0);
        t.insert(String::from("/a/c"), 0);
        t.insert(String::from("/a/d"), 0);
        t.insert(String::from("/a/c/1"), 0);
        t.insert(String::from("/a/c/2"), 0);
        t.insert(String::from("/a/c/2"), 0);
        t.insert(String::from("/a/b/1"), 0);

        assert!(t.children().into_iter().next().is_some());
    }

    #[tokio::test]
    async fn test_stat_mode_fast_does_not_probe_remote_size() {
        use tempfile::tempdir;

        FETCH_FILE_SIZE_CALLS.store(0, Ordering::Relaxed);

        let tmp = tempdir().unwrap();
        let mut store = DictionaryStore::new_with_base_path_and_store_path(
            "/third-party/mega",
            tmp.path().to_str().unwrap(),
        )
        .await;
        // Force fast mode regardless of global config initialization ordering.
        store.stat_mode = config::DicfuseStatMode::Fast;

        // Seed a file entry with a non-empty oid/hash, but with no persisted size.
        store.insert_mock_item(1, 0, "", true).await;
        store.insert_mock_item(2, 1, "foo.txt", false).await;
        let _ = store
            .persistent_path_store
            .update_item_hash(2, "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string());

        let sz = store.file_size_for_stat(2, "deadbeef").await;
        assert_eq!(sz, 0);
        assert_eq!(FETCH_FILE_SIZE_CALLS.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_open_buff_eviction_clears_cache_but_keeps_persisted_content() {
        use tempfile::tempdir;

        let tmp = tempdir().unwrap();
        let mut store = DictionaryStore::new_with_store_path(tmp.path().to_str().unwrap()).await;

        store.open_buff_max_bytes = 10;
        store.open_buff_max_files = 1024;
        store.open_buff_bytes.store(0, Ordering::Release);

        store.save_file(1, b"123456".to_vec()); // 6 bytes
        assert!(store.open_buff.contains_key(&1));

        // Next insert would exceed max_bytes -> evict all, then cache the new one.
        store.save_file(2, b"abcdef".to_vec()); // 6 bytes
        assert!(!store.open_buff.contains_key(&1));
        assert!(store.open_buff.contains_key(&2));

        // Persisted content remains available.
        assert_eq!(
            store.get_persisted_file_content(1).unwrap(),
            b"123456".to_vec()
        );
        assert_eq!(
            store.get_persisted_file_content(2).unwrap(),
            b"abcdef".to_vec()
        );
    }

    /// Helper function to create a DictionaryStore with base_path for testing.
    /// Uses a temporary directory to avoid database lock conflicts in parallel tests.
    async fn create_store_with_base_path_for_test(base_path: &str) -> DictionaryStore {
        use uuid::Uuid;

        // Generate a unique temporary directory for each test
        let test_id = Uuid::new_v4();
        let tmp_dir = format!("/tmp/scorpio_test_{}", test_id);
        std::fs::create_dir_all(&tmp_dir).expect("Failed to create test temp directory");

        let tree_store = TreeStorage::new_with_path(&tmp_dir)
            .expect("Failed to create TreeStorage with temp path");
        let content_store = ContentStorage::new_with_path(&tmp_dir)
            .expect("Failed to create ContentStorage with temp path");
        let size_store = super::super::size_store::SizeStorage::new_with_path(&tmp_dir)
            .expect("Failed to create SizeStorage with temp path");

        let is_subdir_mount = !(base_path.is_empty() || base_path == "/");
        let max_depth = if is_subdir_mount {
            config::antares_load_dir_depth()
        } else {
            config::load_dir_depth()
        };
        let stat_mode = if is_subdir_mount {
            config::antares_dicfuse_stat_mode()
        } else {
            config::dicfuse_stat_mode()
        };
        let dir_sync_ttl = if is_subdir_mount {
            Duration::from_secs(config::antares_dicfuse_dir_sync_ttl_secs())
        } else {
            Duration::from_secs(config::dicfuse_dir_sync_ttl_secs())
        };
        let open_buff_max_bytes = if is_subdir_mount {
            config::antares_dicfuse_open_buff_max_bytes()
        } else {
            config::dicfuse_open_buff_max_bytes()
        };
        let open_buff_max_files = if is_subdir_mount {
            config::antares_dicfuse_open_buff_max_files()
        } else {
            config::dicfuse_open_buff_max_files()
        };

        DictionaryStore {
            next_inode: AtomicU64::new(1),
            inodes: Arc::new(Mutex::new(HashMap::new())),
            radix_trie: Arc::new(Mutex::new(radix_trie::Trie::new())),
            persistent_path_store: Arc::new(tree_store),
            dirs: Arc::new(DashMap::new()),
            dir_locks: Arc::new(DashMap::new()),
            max_depth: Arc::new(max_depth),
            init_notify: Arc::new(Notify::new()),
            ready: AtomicBool::new(false),
            import_started: AtomicBool::new(false),
            persistent_content_store: Arc::new(content_store),
            persistent_size_store: Arc::new(size_store),
            open_buff: Arc::new(DashMap::new()),
            exec_flags: Arc::new(DashMap::new()),
            base_path: base_path.to_string(),
            store_dir: tmp_dir,
            stat_mode,
            dir_sync_ttl,
            open_buff_max_bytes,
            open_buff_max_files,
            open_buff_bytes: AtomicU64::new(0),
        }
    }

    /// Test path conversion methods for subdirectory mounting
    #[tokio::test]
    async fn test_base_path_conversion() {
        // Test with base_path set, using isolated temp database
        let store = create_store_with_base_path_for_test("/third-party/mega").await;

        // Test to_real_path
        assert_eq!(store.to_real_path("/"), "/third-party/mega");
        assert_eq!(store.to_real_path("/src"), "/third-party/mega/src");
        assert_eq!(
            store.to_real_path("/src/main.rs"),
            "/third-party/mega/src/main.rs"
        );

        // Test to_user_path
        assert_eq!(
            store.to_user_path("/third-party/mega"),
            Some("/".to_string())
        );
        assert_eq!(
            store.to_user_path("/third-party/mega/src"),
            Some("/src".to_string())
        );
        assert_eq!(
            store.to_user_path("/third-party/mega/src/main.rs"),
            Some("/src/main.rs".to_string())
        );
        assert_eq!(store.to_user_path("/other/path"), None);
    }

    /// Test path conversion with empty base_path (full monorepo access)
    #[tokio::test]
    async fn test_empty_base_path_conversion() {
        let store = create_store_with_base_path_for_test("").await;

        // With empty base_path, paths should pass through unchanged
        assert_eq!(store.to_real_path("/"), "/");
        assert_eq!(store.to_real_path("/src"), "/src");

        assert_eq!(store.to_user_path("/"), Some("/".to_string()));
        assert_eq!(store.to_user_path("/src"), Some("/src".to_string()));
    }

    /// Test path conversion with root base_path
    #[tokio::test]
    async fn test_root_base_path_conversion() {
        let store = create_store_with_base_path_for_test("/").await;

        // With "/" base_path, paths should pass through unchanged
        assert_eq!(store.to_real_path("/"), "/");
        assert_eq!(store.to_real_path("/src"), "/src");

        assert_eq!(store.to_user_path("/"), Some("/".to_string()));
        assert_eq!(store.to_user_path("/src"), Some("/src".to_string()));
    }

    /// Test base_path with trailing slash
    #[tokio::test]
    async fn test_base_path_with_trailing_slash() {
        let store = create_store_with_base_path_for_test("/third-party/mega/").await;

        // Trailing slash should be handled correctly
        assert_eq!(store.to_real_path("/"), "/third-party/mega");
        assert_eq!(store.to_real_path("/src"), "/third-party/mega/src");
    }
}
