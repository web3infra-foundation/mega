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
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::Notify;

use super::abi::{default_dic_entry, default_file_entry};
use super::content_store::ContentStorage;
use super::tree_store::{StorageItem, TreeStorage};
use crate::util::{config, GPath};
const UNKNOW_INODE: u64 = 0; // illegal inode number;
const INODE_FILE: &str = "file";
const INODE_DICTIONARY: &str = "directory";

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
struct CommitInfoResponse {
    req_result: bool,
    data: Vec<CommitInfo>,
    err_message: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct CommitInfo {
    oid: String,
    name: String,
    content_type: String,
    message: String,
    date: String,
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
struct ApiResponse {
    req_result: bool,
    data: Vec<Item>,
    err_message: String,
}
impl Iterator for ApiResponse {
    type Item = Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.data.pop()
    }
}
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
    static CLIENT: Lazy<Client> = Lazy::new(Client::new);
    let client = CLIENT.clone();
    let url = format!("{}/api/v1/tree?path=/{}", config::base_url(), path);
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

/// Download a file from the server using its OID/hash
async fn fetch_file(oid: &str) -> Vec<u8> {
    let file_blob_endpoint = config::file_blob_endpoint();
    let url = format!("{}/{}", file_blob_endpoint, oid);
    let client = Client::new();

    // Send GET request
    let response = match client.get(url).send().await {
        Ok(resp) => resp,
        Err(_) => {
            eprintln!("Failed to fetch file with OID: {}", oid);
            return Vec::new(); // Return empty vector on error
        }
    };

    // Ensure that the response status is successful
    if response.status().is_success() {
        // Get the binary data from the response body
        let content = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(_) => {
                eprintln!("Failed to read content for OID: {}", oid);
                return Vec::new(); // Return empty vector on error
            }
        };
        return content.to_vec();
    }
    Vec::new()
}

async fn fetch_dir(path: &str) -> Result<ApiResponseExt, DictionaryError> {
    static CLIENT: Lazy<Client> = Lazy::new(Client::new);
    let client = CLIENT.clone();

    let clean_path = path.trim_start_matches('/');
    let url = format!(
        "{}/api/v1/tree/content-hash?path=/{}",
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

    let commit_info: CommitInfoResponse = match response.json().await {
        Ok(info) => info,
        Err(e) => {
            return Err(DictionaryError {
                message: format!("Failed to parse commit info: {}", e),
            });
        }
    };

    if !commit_info.req_result {
        return Err(DictionaryError {
            message: commit_info.err_message,
        });
    }

    let mut data = Vec::with_capacity(commit_info.data.len());

    let base_path = if path.is_empty() || path == "/" {
        "".to_string()
    } else if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{}/", path)
    };

    for info in commit_info.data {
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

    Ok(ApiResponseExt {
        _req_result: true,
        data,
        _err_message: String::new(),
    })
}

/// Get the directory hash from the server
async fn fetch_get_dir_hash(path: &str) -> Result<ApiResponseExt, DictionaryError> {
    static CLIENT: Lazy<Client> = Lazy::new(Client::new);
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

    let commit_info: CommitInfoResponse = match response.json().await {
        Ok(info) => info,
        Err(e) => {
            return Err(DictionaryError {
                message: format!("Failed to parse commit info: {}", e),
            });
        }
    };

    if !commit_info.req_result {
        return Err(DictionaryError {
            message: commit_info.err_message,
        });
    }

    let mut data = Vec::with_capacity(commit_info.data.len());

    let base_path = if path.is_empty() || path == "/" {
        "".to_string()
    } else if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{}/", path)
    };

    for info in commit_info.data {
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
}
pub struct DictionaryStore {
    inodes: Arc<Mutex<HashMap<u64, Arc<DicItem>>>>,
    // dirs: Arc<Mutex<HashMap<String, DirItem>>>, //save all the dirs.
    dirs: Arc<DashMap<String, DirItem>>, // save all the dirs.
    next_inode: AtomicU64,
    radix_trie: Arc<Mutex<radix_trie::Trie<String, u64>>>,
    persistent_path_store: Arc<TreeStorage>, // persistent path store for saving and retrieving file paths
    max_depth: Arc<usize>,                   // max depth for loading directories
    init_notify: Arc<Notify>,                // used in dir_test to notify the start of the test..
    persistent_content_store: Arc<ContentStorage>, // persistent content store for saving and retrieving file contents
    open_buff: Arc<DashMap<u64, Vec<u8>>>,         // buffer for open files
}

#[allow(unused)]
impl DictionaryStore {
    pub async fn new() -> Self {
        let tree_store = TreeStorage::new().expect("Failed to create TreeStorage");
        DictionaryStore {
            next_inode: AtomicU64::new(2),
            inodes: Arc::new(Mutex::new(HashMap::new())),
            radix_trie: Arc::new(Mutex::new(radix_trie::Trie::new())),
            persistent_path_store: Arc::new(tree_store),
            dirs: Arc::new(DashMap::new()),
            max_depth: Arc::new(config::load_dir_depth()),
            init_notify: Arc::new(Notify::new()),
            persistent_content_store: Arc::new(
                ContentStorage::new().expect("Failed to create ContentStorage"),
            ),
            open_buff: Arc::new(DashMap::new()),
        }
    }
    #[inline(always)]
    pub fn max_depth(&self) -> usize {
        *self.max_depth
    }
    pub async fn wait_for_ready(&self) {
        // Wait for the store to be initialized
        self.init_notify.notified().await;
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
                format!("/{}", child_name)
            } else {
                format!("{}/{}", current_path, child_name)
            };

            let relative_path = if base_path == "/" {
                child_full_path
                    .strip_prefix('/')
                    .unwrap_or(&child_full_path)
                    .to_string()
            } else if child_full_path == base_path {
                ".".to_string()
            } else if child_full_path.starts_with(&format!("{}/", base_path)) {
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
            format!("/{}", base_dir)
        } else {
            base_dir.to_string()
        };

        if (self.get_inode_from_path(&normalized_base_dir).await).is_ok() {
            if let Err(e) = self
                .traverse_directory(&normalized_base_dir, &normalized_base_dir, &mut depth_items)
                .await
            {
                println!("Error traversing directory {}: {}", normalized_base_dir, e);
            }

            if normalized_base_dir != "/" {
                depth_items.entry(0).or_default();
            }
        } else {
            println!("Base directory {} not found", normalized_base_dir);
        }

        depth_items
    }

    /// When a file changed,the parent directory's hash changed too.
    /// So we need to update the ancestors' hash .
    pub async fn update_ancestors_hash(&self, inode: u64) {
        let item = self.persistent_path_store.get_item(inode).unwrap();
        let mut parent_inode = item.get_parent();
        while parent_inode != 1 {
            let path = "/".to_string()
                + &self
                    .persistent_path_store
                    .get_all_path(parent_inode)
                    .unwrap()
                    .to_string();
            println!("update hash {:?}", path);
            let hash = get_dir_hash(&path).await;
            if hash.is_empty() {
                return;
            }
            self.persistent_path_store
                .update_item_hash(parent_inode, hash.to_owned())
                .unwrap();
            self.dirs.get_mut(&path).unwrap().hash = hash;
            parent_inode = item.get_parent();
        }
    }
    /// When scorpio start,if the db is not empty, we need to load all the files to the memory.
    fn load_file(&self, inode: u64) -> Result<(), io::Error> {
        let file_content = self.persistent_content_store.get_file_content(inode)?;
        let _ = self.open_buff.insert(inode, file_content);
        Ok(())
    }

    #[async_recursion]
    /// Loads directories recursively from the parent path into memory.
    async fn load_dirs(&self, path: PathBuf, parent_inode: u64) -> Result<(), io::Error> {
        let root_item = self.persistent_path_store.get_item(parent_inode)?;
        self.dirs.insert(
            path.to_string_lossy().to_string(),
            DirItem {
                hash: root_item.hash.to_owned(),
                file_list: HashMap::new(),
            },
        );
        let children = root_item.get_children();
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
            } else {
                self.load_file(child);
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
            println!("fetch path :{}", path);
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
            let mut total_bytes_written = 0;
            let mut current_offset = 0;

            // 4. build a list of StorageItem structs for each child.
            for (i, child) in children.iter().enumerate() {
                re.push(child.clone());
            }
            print!("readdri len :{}", re.len());
            Ok(re)
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "Not a directory"))
        }
    }
}

/// File operations interface for in-memory file management
/// Provides functions to handle file content stored in memory buffer (open_buff)
impl DictionaryStore {
    pub fn get_file_len(&self, inode: u64) -> u64 {
        self.open_buff.get(&inode).map_or(0, |v| v.len() as u64)
    }
    pub fn remove_file_by_node(&self, inode: u64) -> Result<(), io::Error> {
        self.persistent_content_store.remove_file(inode)?;
        self.open_buff.remove(&inode);
        Ok(())
    }
    /// Save to db and then save in the memory.
    pub fn save_file(&self, inode: u64, content: Vec<u8>) {
        self.persistent_content_store
            .insert_file(inode, &content)
            .expect("Failed to save file content");
        self.open_buff.insert(inode, content);
    }
    /// Check if the file exists in the memory.
    pub fn file_exists(&self, inode: u64) -> bool {
        self.open_buff.contains_key(&inode)
    }
    /// Get the file content from the memory.
    pub fn get_file_content(&self, inode: u64) -> Option<Ref<'_, u64, Vec<u8>>> {
        self.open_buff.get(&inode)
    }

    /// Doanload the file content from the server and save it to the db and memory.
    pub async fn fetch_file_content(&self, inode: u64, oid: &str) {
        let content = fetch_file(oid).await;
        self.save_file(inode, content);
    }
    /// Return the content of a file by its path.
    pub async fn get_file_content_by_path(&self, path: &str) -> Result<Vec<u8>, io::Error> {
        let inode = self.get_inode_from_path(path).await?;
        if let Some(content) = self.get_file_content(inode) {
            Ok(content.to_vec())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "File not found"))
        }
    }
}
/// Loads subdirectories from a remote server into an empty parent directory up to a specified depth.
///
/// # Arguments
/// * `parent_path` - The path to an empty directory where subdirectories will be loaded.
/// * `max_depth` - The maximum absolute depth of subdirectories to load, relative to the root.
pub async fn load_dir_depth(store: Arc<DictionaryStore>, parent_path: String, max_depth: usize) {
    println!("load_dir_depth {:?}", parent_path);
    let queue = Arc::new(SegQueue::new());
    let items = fetch_dir(&parent_path).await.unwrap().data;
    // only count the directories.
    let dir_count = items.iter().filter(|it| it.item.is_dir()).count();
    let active_producers = Arc::new(AtomicUsize::new(dir_count));
    // let active_producers = Arc::new(AtomicUsize::new(items.len()));
    {
        let locks = store.dirs.clone();
        for it in items {
            let is_dir = it.item.is_dir();
            let path = it.item.path.to_owned();
            locks
                .get_mut(&parent_path)
                .unwrap()
                .file_list
                .insert(path.to_owned(), false);
            let parent_node = store.get_inode_from_path(&parent_path).await.unwrap();
            let it_inode = store.update_inode(parent_node, it.clone()).await.unwrap();
            if is_dir {
                queue.push(it_inode);
                locks.insert(
                    path,
                    DirItem {
                        hash: it.hash,
                        file_list: HashMap::new(),
                    },
                );
            } else {
                store.fetch_file_content(it_inode, it.hash.as_str()).await;
            }
        }
    }

    let worker_count = 10;
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
            while {
                // If there are active producers or the queue is not empty, continue
                producers.load(Ordering::Acquire) > 0 || !queue.is_empty()
            } {
                if let Some(inode) = queue.pop() {
                    //get the whole path.
                    let path =
                        "/".to_string() + &path_store.get_all_path(inode).unwrap().to_string();
                    println!("Worker processing path: {}", path);
                    // get all children inode
                    match fetch_dir(&path).await {
                        Ok(new_items) => {
                            let new_items = new_items.data;
                            for newit in new_items {
                                let is_dir = newit.item.is_dir();
                                let tmp_path = newit.item.path.to_owned();
                                store
                                    .dirs
                                    .get_mut(&path)
                                    .unwrap()
                                    .file_list
                                    .insert(tmp_path.to_owned(), false);
                                let new_inode =
                                    store.update_inode(inode, newit.clone()).await.unwrap();
                                if is_dir {
                                    // If it's a directory, push it to the queue and add the producer count
                                    if tmp_path.matches('/').count() < max_depth {
                                        producers.fetch_add(1, Ordering::Relaxed);
                                        queue.push(new_inode);
                                    } else {
                                        println!("max_depth reach path = {:?}", tmp_path);
                                    }
                                    store.dirs.insert(
                                        tmp_path,
                                        DirItem {
                                            hash: newit.hash,
                                            file_list: HashMap::new(),
                                        },
                                    );
                                } else {
                                    // If it's a file, fetch its content
                                    store
                                        .fetch_file_content(new_inode, newit.hash.as_str())
                                        .await;
                                }
                            }
                        }
                        Err(_) => {
                            // Continue to the next iteration if there was an error
                        }
                    };

                    producers.fetch_sub(1, Ordering::Release);
                } else {
                    // If there are no active producers and the queue is empty, exit the loop
                    if producers.load(Ordering::Acquire) == 0 {
                        return;
                    }
                    // yield to wait unfinished tasks
                    tokio::task::yield_now().await;
                }
            }
        }));
    }

    // wait for all workers to complete
    // while let Some(worker) = workers.pop() {
    //     worker.await.expect("Worker panicked");
    // }
    join_all(workers).await;
}

pub async fn import_arc(store: Arc<DictionaryStore>) {
    //first load the db.
    if store.load_db().await.is_ok() {
        store.init_notify.notify_waiters();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                watch_dir(store.clone()).await;
            }
        });
        return;
    } else {
        //if the db is null,then init the store and load from mono.
        let _ = store.persistent_path_store.insert_item(
            1,
            UNKNOW_INODE,
            ItemExt {
                item: Item {
                    name: "".to_string(),
                    path: "/".to_string(),
                    content_type: INODE_DICTIONARY.to_string(),
                },
                hash: String::new(),
            },
        );
        let root_item = DicItem {
            inode: 1,
            path_name: GPath::new(),
            content_type: Arc::new(Mutex::new(ContentType::Directory(false))),
            children: Mutex::new(HashMap::new()),
            parent: UNKNOW_INODE, //  root dictory has no parent
        };
        let root_dir_item = DirItem {
            hash: String::new(),
            file_list: HashMap::new(),
        };
        store.inodes.lock().await.insert(1, root_item.into());
        store.dirs.insert("/".to_string(), root_dir_item);
    }

    let max_depth = store.max_depth() + 2;
    load_dir_depth(store.clone(), "/".to_string(), max_depth).await;
    store.init_notify.notify_waiters();
    // use the unlock queue instead of mpsc  Mutex
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            watch_dir(store.clone()).await;
        }
    });
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
pub async fn load_dir(store: Arc<DictionaryStore>, parent_path: String, max_depth: usize) -> bool {
    if parent_path.matches('/').count() >= max_depth {
        println!("max depth reached for path: {}", parent_path);
        return false;
    }
    if max_depth < store.max_depth() + 2 {
        println!("max depth is less than config, skipping: {}", parent_path);
        return false;
    }

    let parent_inode = store.get_inode_from_path(&parent_path).await.unwrap();

    let tree_db = store.persistent_path_store.clone();
    let dirs = store.dirs.clone();
    let self_hash = get_dir_hash(&parent_path).await;

    //the dir may be deleted.
    if self_hash.is_empty() {
        println!("Directory {} is empty, no items to load.", parent_path);
        return true;
    }
    println!("load_dir parent_path {:?}", parent_path);

    //empty dir,load the dir to the max_depth.
    if dirs.get(&parent_path).unwrap().file_list.is_empty() {
        load_dir_depth(store.clone(), parent_path.to_owned(), max_depth).await;

        if dirs.get(&parent_path).unwrap().hash != self_hash {
            dirs.get_mut(&parent_path).unwrap().hash = self_hash.to_owned();
            let inode = store.get_inode_from_path(&parent_path).await.unwrap();
            tree_db.update_item_hash(inode, self_hash).unwrap();
            return true;
        }
        return false;
    }
    // if the dir's hash is same as the parent dir's hash,
    //then check the subdir from the db,no need to get from the server..
    if dirs.get(&parent_path).unwrap().hash == self_hash {
        let item = store.persistent_path_store.get_item(parent_inode).unwrap();
        let children = item.get_children();
        for child in children {
            let child_item = store.persistent_path_store.get_item(child).unwrap();
            if child_item.is_dir() {
                println!(
                    "handle dir /{:?}",
                    tree_db.get_all_path(child).unwrap().to_string()
                );
                load_dir(
                    store.clone(),
                    "/".to_string() + &tree_db.get_all_path(child).unwrap().to_string(),
                    max_depth,
                )
                .await;
            }
        }
        return false;
    }
    //last, if the dir's hash is different from the parent dir's hash,
    //then fetch the dir from the server.
    let items = fetch_dir(&parent_path).await.unwrap().data;
    dirs.get_mut(&parent_path).unwrap().hash = self_hash.to_owned();
    let inode = store.get_inode_from_path(&parent_path).await.unwrap();
    tree_db.update_item_hash(inode, self_hash).unwrap();
    for it in items {
        let is_dir = it.item.is_dir();
        let path = it.item.path.to_owned();

        // the item already exists in the parent directory.
        if dirs
            .get(&parent_path)
            .unwrap()
            .file_list
            .contains_key(&path)
        {
            dirs.get_mut(&parent_path)
                .unwrap()
                .file_list
                .insert(path.to_owned(), true);
            if is_dir {
                println!("hash changes dir {:?}", path);
                load_dir(store.clone(), path.to_owned(), max_depth).await;
            } else {
                let inode = store.get_inode_from_path(&path).await.unwrap();
                let item = store.persistent_path_store.get_item(inode).unwrap();
                if item.hash != it.hash {
                    // update the hash in the db.
                    tree_db.update_item_hash(inode, it.hash.to_owned()).unwrap();
                    store.fetch_file_content(inode, &it.hash).await
                }
            }
        } else {
            dirs.get_mut(&parent_path)
                .unwrap()
                .file_list
                .insert(path.to_owned(), true);
            println!("load dir add new file {:?}", path);
            let new_node = store.update_inode(parent_inode, it.clone()).await.unwrap();
            //fetch a new dir.
            if is_dir {
                println!("add dir {:?}", path);
                dirs.insert(
                    path.to_owned(),
                    DirItem {
                        hash: it.hash,
                        file_list: HashMap::new(),
                    },
                );
                load_dir_depth(store.clone(), path.to_owned(), max_depth).await;
            } else {
                store.fetch_file_content(new_node, &it.hash).await
            }
        }
    }
    let mut remove_items = Vec::new();
    dirs.get_mut(&parent_path)
        .unwrap()
        .file_list
        .retain(|path, v| {
            let result = *v;
            if !(*v) {
                remove_items.push(path.clone());
            } else {
                *v = false;
            }
            result
        });
    for item in remove_items {
        let inode = store.get_inode_from_path(&item).await.unwrap();
        println!("delete {:?} {} ", inode, item);
        tree_db.remove_item(inode).unwrap();
        let _ = store.remove_file_by_node(inode);
    }
    return true;
}

#[async_recursion]
/// This function is only used to update the directory which has been loaded.
/// It will update the directory but do not load the new directory.
pub async fn update_dir(store: Arc<DictionaryStore>, parent_path: String) {
    let tree_db = store.persistent_path_store.clone();
    let items = fetch_dir(&parent_path).await.unwrap().data;
    let dirs = store.dirs.clone();

    for it in items {
        let is_dir = it.item.is_dir();
        let path = it.item.path.to_owned();

        // the item already exists in the parent directory.
        if dirs
            .get(&parent_path)
            .unwrap()
            .file_list
            .contains_key(&path)
        {
            dirs.get_mut(&parent_path)
                .unwrap()
                .file_list
                .insert(path.to_owned(), true);

            let inode = store.get_inode_from_path(&path).await.unwrap();
            let item = store.persistent_path_store.get_item(inode).unwrap();
            if item.hash != it.hash {
                if is_dir {
                    //when the dir's hash changed,fetch the dir.
                    // If the path already exists, update the hash
                    update_dir(store.clone(), path.to_owned()).await;

                    let mut dir_it = dirs.get_mut(&path).unwrap();
                    dir_it.hash = it.hash.to_owned();
                    //also update the hash in the db.

                    println!("modify dir {:?}", path);
                } else {
                    // If it's a file, fetch its content
                    // update the hash in the db.
                    store.fetch_file_content(inode, &it.hash).await
                }
                tree_db.update_item_hash(inode, it.hash).unwrap();
            }
        } else {
            dirs.get_mut(&parent_path)
                .unwrap()
                .file_list
                .insert(path.to_owned(), true);
            println!("update_dir new add file {:?}", path);
            let parent_inode = store.get_inode_from_path(&parent_path).await.unwrap();

            let new_node = store.update_inode(parent_inode, it.clone()).await.unwrap();
            //fetch a new dir.
            if is_dir {
                println!("add dir {:?}", path);
                dirs.insert(
                    path,
                    DirItem {
                        hash: it.hash,
                        file_list: HashMap::new(),
                    },
                );
            } else {
                // If it's a file, fetch its content
                store.fetch_file_content(new_node, &it.hash).await;
            }
        }
    }

    let mut remove_items = Vec::new();
    dirs.get_mut(&parent_path)
        .unwrap()
        .file_list
        .retain(|path, v| {
            let result = *v;
            if !(*v) {
                remove_items.push(path.clone());
            } else {
                *v = false;
            }
            result
        });
    for item in remove_items {
        let inode = store.get_inode_from_path(&item).await.unwrap();
        println!("delete {:?} {} ", inode, item);
        tree_db.remove_item(inode).unwrap();
        let _ = store.remove_file_by_node(inode);
    }
}

/// Watch the directory and update the dictionary has loaded.
pub async fn watch_dir(store: Arc<DictionaryStore>) {
    update_dir(store, "/".to_string()).await;
}

#[cfg(test)]
mod tests {
    use radix_trie::TrieCommon;

    use super::*;
    #[tokio::test]
    #[ignore]
    async fn test_fetch_tree_success() {
        let path: &str = "/third-party/mega";

        let result = fetch_tree(path).await.unwrap();
        println!("result: {:?}", result);
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

        let c = t.children();
        c.into_iter().for_each(|it| println!("{:?}\n", it))
    }
}
