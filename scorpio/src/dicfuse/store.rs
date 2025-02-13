
use fuse3::raw::reply::ReplyEntry;
use fuse3::FileType;

/// Read only file system for obtaining and displaying monorepo directory information
use reqwest::Client;
// Import Response explicitly
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use std::io;


use std::{collections::HashMap, error::Error};
use std::collections::VecDeque;
use once_cell::sync::Lazy;
use radix_trie::{self, TrieCommon};
use std::sync::Arc;
use crate::util::atomic::AtomicU64;
use crate::READONLY_INODE;

use super::abi::{default_dic_entry, default_file_entry};
use super::tree_store::TreeStorage;
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
    queue: Arc<Mutex<VecDeque<u64>>>,
    radix_trie: Arc<Mutex<radix_trie::Trie<String, u64>>>,
    persistent_path_store :Arc<Mutex<TreeStorage>>,// persistent path store for saving and retrieving file paths
}

#[allow(unused)]
impl DictionaryStore {
    #[allow(clippy::await_holding_lock)]
    pub async fn async_import(&self){
    
            let items = fetch_tree("").await.unwrap().data.clone() ;

            let root_inode: Arc<DicItem> = self.inodes.lock().await.get(&1).unwrap().clone();
            for it in items{
                println!("root item:{:?}",it);
                self.update_inode(root_inode.clone(),it).await;
            }
            loop {//BFS to look up all dictionary
                if self.queue.lock().await.is_empty(){
                    break;
                }
                let one_inode = self.queue.lock().await.pop_front().unwrap();
                let mut new_items = Vec::new();
                {
                    let it = self.inodes.lock().await.get(&one_inode).unwrap().clone();
                    let mut ct =it.content_type.lock().await;
                    let path=String::new();
                    if let ContentType::Directory(load) = *ct{
                        if !load{
                            *ct = ContentType::Directory(true);
                            let path = it.get_path();
                            println!("fetch path :{}",path);
                        }
                        
                        if path.len()>1{
                            drop(ct);
                            let t = fetch_tree(&path.clone()).await;
                            new_items = t.unwrap().data.clone() ;
                        }
                    }

                   
                    let mut pc = it.clone();
                    for newit in new_items {
                        println!("import item :{:?}",newit);
                        self.update_inode(pc.clone(),newit).await; // Await the update_inode call
                    }
                    
                }
                new_items = Vec::new();
            }
            //queue.clear();
        
    }
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
            queue: Arc::new(Mutex::new(VecDeque::new())),
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
    async fn  update_inode(&self,pitem:Arc<DicItem>,item:Item){
        self.next_inode.fetch_add(1).await;
        
        let alloc_inode = self.next_inode.load().await;
        
        assert!(alloc_inode < READONLY_INODE );
        if item.content_type==INODE_DICTIONARY{
            self.queue.lock().await.push_back(alloc_inode);
        }
        
        
        let parent = pitem;
        let newitem = Arc::new(DicItem::new(alloc_inode, parent.get_inode(),item.clone()));
       
        parent.push_children(newitem.clone()).await;
        self.radix_trie.lock().await.insert(newitem.get_path(), alloc_inode);
        self.inodes.lock().await.insert(alloc_inode, newitem);
        self.persistent_path_store.lock().await.insert_item(alloc_inode, parent.get_inode(), item);

    }

    pub async fn import(&self){
        // 在阻塞线程中运行异步任务
        let items =  fetch_tree("").await.unwrap().data;
        
        let root_inode = self.inodes.lock().await.get(&1).unwrap().clone();


        for it in items{
            println!("root item:{:?}",it);
            self.update_inode(root_inode.clone(),it).await;
        }
        loop {//BFS to look up all dictionary
            if self.queue.lock().await.is_empty(){
                break;
            }
            let one_inode = self.queue.lock().await.pop_front().unwrap();
            let mut new_items = Vec::new();
            {
                let it = self.inodes.lock().await.get(&one_inode).unwrap().clone();
                if let ContentType::Directory(load) = *it.content_type.lock().await{
                    if !load{
                        let path = it.get_path();
                        println!("fetch path :{}",path);
                        
                        // 在阻塞线程中运行异步任务
                        new_items =fetch_tree(&path).await.unwrap().data;
                
                    }
                   
                }
                let mut pc = it.clone();
                for newit in new_items {
                    println!("import item :{:?}",newit);
                    self.update_inode(pc.clone(),newit).await; // Await the update_inode call
                }
                let mut content_type = pc.content_type.lock().await;
                *content_type = ContentType::Directory(true);
            }
            new_items = Vec::new();
        }
        //queue.clear();
    }
    
    pub async fn find_path(&self,inode :u64)-> Option<GPath>{
        self.inodes.lock().await.get(&inode).map(|item| item.path_name.clone())
    }
    pub async fn get_inode(&self,inode: u64) -> Result<Arc<DicItem>, io::Error> {
        match self.inodes.lock().await.get(&inode) {
            Some(item) => Ok(item.clone()),
            None=>Err(io::Error::new(io::ErrorKind::NotFound, "inode not found"))
        }
       
    }
    pub async fn get_by_path(&self, path: &str) -> Result<Arc<DicItem>, io::Error> {
        let binding = self.radix_trie.lock().await;
        let inode = binding.get(path).ok_or(io::Error::new(io::ErrorKind::NotFound, "path not found"))?;
        self.get_inode(*inode).await
    }
    async fn find_children(&self,parent: u64) -> Result<DicItem,io::Error>{
        let path = self.inodes.lock().await.get(&parent).map(|item| item.path_name.clone());
        if let Some(parent_path) = path{
            let l  = self.radix_trie.lock().await;
            let pathstr:String =parent_path.name();
            let u = l.subtrie(&pathstr).unwrap();
            let c = u.children();
        }
        todo!()
    }
    pub async fn do_readdir(&self,parent:u64,fh:u64,offset:u64) -> Result<Vec<Arc<DicItem>>, io::Error>{
        //  1. 获取目录项
        let dictionary = self.get_inode(parent).await?;
        let p_dictionary = self.get_inode(dictionary.get_inode()).await?;
        let mut re = vec![dictionary.clone(),p_dictionary.clone()];
        //let mut re = vec![];
         // 2. 确保目录项是一个目录
         if let ContentType::Directory(_) = dictionary.get_tyep().await {
             // 3. 获取子目录项
             let  children = dictionary.children.lock().await;
             let mut total_bytes_written = 0;
             let mut current_offset = 0;

             // 4. 遍历子目录项
             for (i, (name, child)) in children.iter().enumerate() {
                re.push(child.clone());
             }
             Ok(re)
         } else {
             Err(io::Error::new(io::ErrorKind::NotFound, "Not a directory"))
         }
    }
}
#[cfg(test)]
mod tests {
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

