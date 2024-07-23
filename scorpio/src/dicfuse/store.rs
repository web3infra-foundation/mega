
/// Read only file system for obtaining and displaying monorepo directory information
use reqwest::Client;
// Import Response explicitly
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::atomic::AtomicU64;
use std::{collections::HashMap, error::Error};
use std::collections::VecDeque;
use once_cell::sync::Lazy;
use radix_trie::{self, TrieCommon};
use std::sync::{Arc,Mutex};


use super::model::GPath;
const MEGA_TREE_URL: &str = "localhost:8000";//TODO: make it configable


#[derive(Serialize, Deserialize, Debug)]
pub struct Item {
    name: String,
    path: String,
    content_type: String,
}
#[allow(unused)]
struct DicItem{
    inode:u64,
    name:GPath,
    content_type: ContentType,
}

#[allow(unused)]
#[derive(PartialEq)]
enum ContentType {
    File,
    Dictionary,
}
#[allow(unused)]
impl DicItem {
    pub fn new(inode:u64, item:Item) -> Self {
        DicItem {
            inode,
            name: item.name.into(), // Assuming GPath can be created from String
            content_type: match item.content_type.as_str() {
                "file" => ContentType::File,
                "directory" => ContentType::Dictionary,
                _ => panic!("Unknown content type"),
            },
        }
    }
    pub fn get_path(&self) -> String {
        self.name.name()
    }
}
#[derive(Serialize, Deserialize, Debug,Default)]
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
    let url = format!("http://{}/api/v1/tree?path={}", MEGA_TREE_URL, path);
    let  resp:ApiResponse = client.get(&url).send().await?.json().await?;
    if resp.req_result {   
        Ok(resp)
    }else{
        todo!();
    }
}

#[allow(unused)]
pub struct DictionaryStore {
    inodes: Arc<Mutex<HashMap<u64, DicItem>>>,
    next_inode: AtomicU64,
    queue: Arc<Mutex<VecDeque<u64>>>,
    radix_trie: Arc<Mutex<radix_trie::Trie<String, u64>>>,
}


#[allow(unused)]
impl DictionaryStore {
    pub fn new() -> Self {
        DictionaryStore {
            next_inode: AtomicU64::new(1),
            inodes: Arc::new(Mutex::new(HashMap::new())),
            radix_trie: Arc::new(Mutex::new(radix_trie::Trie::new())),
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    fn update_inode(&self,item:Item){
        self.next_inode.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let alloc_inode = self.next_inode.load(std::sync::atomic::Ordering::Relaxed);
        self.radix_trie.lock().unwrap().insert(item.path.clone(), alloc_inode);
        self.inodes.lock().unwrap().insert(alloc_inode, DicItem::new(alloc_inode, item));
        self.queue.lock().unwrap().push_back(alloc_inode);
    }
    pub fn import(&self){
        const ROOT_DIR: &str ="/";
        let mut queue = VecDeque::new();
        let items: Vec<Item> = tokio::runtime::Runtime::new().unwrap().block_on(fetch_tree(ROOT_DIR)).unwrap().collect();//todo: can't tokio
        for it in items{
            self.update_inode(it);
        }
        while !queue.is_empty() {//BFS to look up all dictionary
            let one_inode = queue.pop_back().unwrap();
            let mut new_items = Vec::new();
            {
                let inodes_lock = self.inodes.lock().unwrap();
                let it = inodes_lock.get(&one_inode).unwrap();
                if it.content_type == ContentType::Dictionary{
                    let path = it.get_path();
                    new_items = tokio::runtime::Runtime::new().unwrap().block_on(fetch_tree(&path)).unwrap().collect();
                }
            }
            for newit in new_items {
                self.update_inode(newit); // Await the update_inode call
            }
            new_items = Vec::new();
        }
        //queue.clear();
    }

    
    pub fn find_path(&self,inode :u64)-> Option<GPath>{
        self.inodes.lock().unwrap().get(&inode).map(|item| item.name.clone())
    }

    fn find_children(&self,parent: u64) -> Result<DicItem,io::Error>{
        let path = self.inodes.lock().unwrap().get(&parent).map(|item| item.name.clone());
        if let Some(parent_path) = path{
            let l  = self.radix_trie.lock().unwrap();
            let pathstr:String =parent_path.name();
            let u = l.subtrie(&pathstr).unwrap();
            let c = u.children();
        }
        todo!()
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

