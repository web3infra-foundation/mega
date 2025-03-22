
use fuse3::raw::reply::ReplyEntry;
use fuse3::FileType;

/// Read only file system for obtaining and displaying monorepo directory information
use reqwest::Client;
// Import Response explicitly
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use core::panic;
use std::io;
use std::{collections::HashMap, error::Error};
use std::collections::VecDeque;
use once_cell::sync::Lazy;
use std::sync::Arc;

use crate::READONLY_INODE;
use std::sync::atomic::AtomicU64;

use super::abi::{default_dic_entry, default_file_entry};
use super::tree_store::{StorageItem, TreeStorage};
use crate::util::GPath;
const MEGA_TREE_URL: &str = "localhost:8000";//TODO: make it configable
const UNKNOW_INODE: u64 = 0; // illegal inode number;
const INODE_FILE :&str ="file";
const INODE_DICTIONARY :&str ="directory";

#[derive(Serialize, Deserialize, Debug,Clone, Default,PartialEq)]
pub struct Item {
    pub name: String,
    pub path: String,
    pub content_type: String,
}
impl Item {
    pub fn is_dir(&self)->bool{
        self.content_type == INODE_DICTIONARY
    }
}
#[allow(unused)]
pub struct DicItem{
    inode:u64,
    path_name:GPath,
    content_type: Arc<Mutex<ContentType>>,
    pub children:Mutex<HashMap<String, Arc<DicItem>>>,
    parent:u64,
}

#[allow(unused)]
#[derive(PartialEq,Debug)]
enum ContentType {
    File,
    Directory(bool),// if this dictionary is loaded.
}
#[allow(unused)]
impl DicItem {
    pub fn new(inode:u64,parent:u64, item:Item) -> Self {
        DicItem {
            inode,
            path_name: item.path.into(), // GPath can be created from String
            content_type: match item.content_type.as_str() {
                INODE_FILE =>Arc::new(Mutex::new(ContentType::File)),
                INODE_DICTIONARY =>Arc::new(Mutex::new(ContentType::Directory(false))),
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
    pub async fn push_children(&self,children:Arc<DicItem>){
        self.children.lock().await.insert(children.get_name(), children);
    }
    // get the inode 
    pub fn get_inode(&self)-> u64{
        self.inode
    }
    async fn get_tyep(&self) -> ContentType{
        let t  = self.content_type.lock().await;
        match *t{
            ContentType::File => ContentType::File,
            ContentType::Directory(a) => ContentType::Directory(a),
        }
    }
    pub async fn get_filetype(&self)-> FileType{
        let t  = self.content_type.lock().await;
        match *t{
            ContentType::File => FileType::RegularFile,
            ContentType::Directory(_) => FileType::Directory,
        }
    }
    pub fn get_parent(&self)-> u64{
        self.parent
    }
    pub async fn get_stat(&self) ->ReplyEntry{
        match self.get_tyep().await{
            ContentType::File => default_file_entry(self.inode),
            ContentType::Directory(_) => default_dic_entry(self.inode),
        }
    }
}


#[derive(Serialize, Deserialize, Debug,Default,Clone)]
struct ApiResponse {
    req_result: bool,
    data: Vec<Item>,
    err_message: String,
}
impl Iterator for ApiResponse{
    type Item = Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.data.pop()
    }
}
// Get Mega dictionary tree from server
async fn fetch_tree(path: &str) -> Result<ApiResponse, Box<dyn Error>> {
    static CLIENT: Lazy<Client> = Lazy::new(Client::new);
    let client = CLIENT.clone();
    let url = format!("http://{}/api/v1/tree?path=/{}", MEGA_TREE_URL, path);
    let  resp:ApiResponse = client.get(&url).send().await?.json().await?;
    if resp.req_result {   
        Ok(resp)
    }else{
        todo!();
    }
}


pub struct DictionaryStore {
    
    inodes: Arc<Mutex<HashMap<u64, Arc<DicItem>>>>,
    next_inode: AtomicU64,
    radix_trie: Arc<Mutex<radix_trie::Trie<String, u64>>>,
    persistent_path_store :Arc<Mutex<TreeStorage>>,// persistent path store for saving and retrieving file paths
}

#[allow(unused)]
impl DictionaryStore {
    
    pub async fn new() -> Self {
        let tree_store =  TreeStorage::new().expect("Failed to create TreeStorage");
        tree_store.insert_item(1, UNKNOW_INODE, Item{
            name: "".to_string(),
            path: "/".to_string(),
            content_type: INODE_DICTIONARY.to_string(),
        });
        let mut init = DictionaryStore {
            next_inode: AtomicU64::new(2),
            inodes: Arc::new(Mutex::new(HashMap::new())),
            radix_trie: Arc::new(Mutex::new(radix_trie::Trie::new())),
            persistent_path_store:  Arc::new(Mutex::new(tree_store))
        };
        let root_item = DicItem{
            inode: 1,
            path_name: GPath::new(),
            content_type: Arc::new(Mutex::new(ContentType::Directory(false))),
            children: Mutex::new(HashMap::new()),
            parent: UNKNOW_INODE, //  root dictory has no parent
        };
        
        init.inodes.lock().await.insert(1, root_item.into());
        init
    }
    async fn update_inode(&self,parent: u64,item:Item) ->std::io::Result<u64> {
        self.next_inode.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        let alloc_inode = self.next_inode.load(std::sync::atomic::Ordering::SeqCst);
        
        assert!(alloc_inode < READONLY_INODE );
        
        let prw = self.persistent_path_store.lock().await;
        if let Ok(pinode) = prw.get_item(parent){
            // insert info to a radix_trie for path match.
            self.radix_trie.lock().await.insert(GPath::from(item.path.clone()).to_string(), alloc_inode);
            prw.insert_item(alloc_inode, parent, item);
            //prw.append_child(parent, alloc_inode);
        }else{
            //error...
            return Err(io::Error::new(io::ErrorKind::NotFound, "Parent inode not found"));
        }

        Ok(alloc_inode)
    }

    pub async fn add_temp_point(&self,path:&str)-> Result<u64,io::Error>{
        let item_path = path.to_string();
        let mut path = GPath::from(path.to_string());
        let name =  path.pop();
        let parent = self.get_by_path(&path.to_string()).await?;
        let name = match name {
            Some(n) => n,
            None => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid path")),
        };
        self.update_inode(parent.get_inode(), Item{
            name,
            path: item_path,
            content_type: INODE_DICTIONARY.to_string(),
        }).await

    }

    pub async fn import(&self){
        
        let items =  fetch_tree("").await.unwrap().data;
        
        let root_inode = self.inodes.lock().await.get(&1).unwrap().clone();
        // deque for bus.
        let mut queue= VecDeque::<u64>::new(); 
        for it in items{
            let is_dir = it.content_type ==INODE_DICTIONARY;
            let it_inode = self.update_inode(1,it).await.unwrap();
            if is_dir{
                queue.push_back(it_inode);
            }
        }
        
        loop {//BFS to look up all dictionary
            if queue.is_empty(){
                break;
            }
            let one_inode = queue.pop_front().unwrap();
            let mut new_items = Vec::new();
            
            let it = self.persistent_path_store.lock().await.get_all_path(one_inode).unwrap();
            let path = it.to_string();
            println!("fetch path :{}",path);
            // get tree by parent inode.
            new_items =fetch_tree(&path).await.unwrap().data;
            
            // Insert all new inode.
            for newit in new_items {
                println!("import item :{:?}",newit);
                let is_dir = newit.is_dir();
                let new_inode = self.update_inode(one_inode,newit).await.unwrap(); // Await the update_inode call
                // push to queue to BFS.
                if is_dir{
                    queue.push_back(new_inode);
                }
            }
        
        }
        //queue.clear();
    }
    
    pub async fn find_path(&self,inode :u64)-> Option<GPath>{

        self.persistent_path_store.lock().await.get_all_path(inode).ok()

    }
    pub async fn get_inode(&self,inode: u64) -> Result<StorageItem, io::Error> {
        self.persistent_path_store.lock().await.get_item(inode)
    }
    
    pub async fn get_by_path(&self, path: &str) -> Result<StorageItem, io::Error> {
        let inode = 
        if path.is_empty() || path=="/"{
             1
        }else{
            let binding = self.radix_trie.lock().await;
            *binding
                .get(path)
                .ok_or(io::Error::new(io::ErrorKind::NotFound, "path not found"))?
               
        };
        
        self.get_inode(inode).await
    }

    pub async fn do_readdir(&self,parent:u64,fh:u64,offset:u64) -> Result<Vec<StorageItem>, io::Error>{
        //  1. get the parent directory.
        let  item = self.get_inode(parent).await?; // current_dictionary
        let  mut parent_path = self.find_path(parent).await.unwrap();
        parent_path.pop();

        let parent_item =self.get_by_path(&parent_path.to_string()).await?;

        
        let mut re = vec![item.clone(),parent_item.clone()];
        
        // 2. make sure this item is a directory
         if item.is_dir(){
             // 3. Get the children of the directory
             
             let children = self.persistent_path_store.lock().await.get_children(parent)?;
             let mut total_bytes_written = 0;
             let mut current_offset = 0;

             // 4. build a list of StorageItem structs for each child.
             for (i,  child) in children.iter().enumerate() {
                re.push(child.clone());
             }
             print!("readdri len :{}",re.len());
             Ok(re)
         } else {
             Err(io::Error::new(io::ErrorKind::NotFound, "Not a directory"))
         }
    }
}
#[cfg(test)]
mod tests {
    use radix_trie::TrieCommon;

    use super::*;
    #[tokio::test]
    #[ignore] // This will prevent the test from running by default
    async fn test_fetch_tree_success() {
        let path: &str = "/third-part/mega";

        let result = fetch_tree(path).await.unwrap();
        println!("result: {:?}", result);

    }

    #[test]
    fn test_tree(){
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
            c.into_iter().for_each(|it|println!("{:?}\n",it)
        )
    }
}

