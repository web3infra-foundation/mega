
use fuse_backend_rs::api::filesystem::{Context, Entry};
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
use fuse_backend_rs::api::filesystem::DirEntry;
use crate::fuse::READONLY_INODE;
use crate::get_handle;

use super::fuse::{self, default_dic_entry, default_file_entry};
use crate::util::GPath;
const MEGA_TREE_URL: &str = "localhost:8000";//TODO: make it configable
const UNKNOW_INODE: u64 = 0; // illegal inode number;
const INODE_FILE :&str ="file";
const INODE_DICTIONARY :&str ="directory";

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Item {
    name: String,
    path: String,
    content_type: String,
}
#[allow(unused)]
pub struct DicItem{
    inode:u64,
    path_name:GPath,
    content_type: Arc<Mutex<ContentType>>,
    children:Mutex<HashMap<String, Arc<DicItem>>>,
    parent:u64,
}

#[allow(unused)]
#[derive(PartialEq,Debug)]
enum ContentType {
    File,
    Dictionary(bool),// if this dictionary is loaded.
}
#[allow(unused)]
impl DicItem {
    pub fn new(inode:u64,parent:u64, item:Item) -> Self {
        DicItem {
            inode,
            path_name: item.path.into(), // GPath can be created from String
            content_type: match item.content_type.as_str() {
                INODE_FILE =>Arc::new(Mutex::new(ContentType::File)),
                INODE_DICTIONARY =>Arc::new(Mutex::new(ContentType::Dictionary(false))),
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
    pub fn push_children(&self,children:Arc<DicItem>){
        self.children.lock().unwrap().insert(children.get_name(), children);
    }
    // get the inode 
    pub fn get_inode(&self)-> u64{
        self.inode
    }
    fn get_tyep(&self) -> ContentType{
        let t  = self.content_type.lock().unwrap();
        match *t{
            ContentType::File => ContentType::File,
            ContentType::Dictionary(a) => ContentType::Dictionary(a),
        }
    }
    pub fn  get_stat(&self) ->Entry{
        match self.get_tyep(){
            ContentType::File => default_file_entry(self.inode),
            ContentType::Dictionary(_) => default_dic_entry(self.inode),
        }
    }
}

pub trait IntoEntry {
    fn into_entry(self) -> Entry;
}

impl IntoEntry for Arc<DicItem> {
    fn into_entry(self) -> Entry {
        match *self.content_type.lock().unwrap() {
            ContentType::File => fuse::default_file_entry(self.inode),
            ContentType::Dictionary(_) => fuse::default_dic_entry(self.inode),
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

#[allow(unused)]
pub struct DictionaryStore {
    inodes: Arc<Mutex<HashMap<u64, Arc<DicItem>>>>,
    next_inode: AtomicU64,
    queue: Arc<Mutex<VecDeque<u64>>>,
    radix_trie: Arc<Mutex<radix_trie::Trie<String, u64>>>,
}

#[allow(unused)]
impl DictionaryStore {
    pub async fn async_import(&self){
    
            let items = fetch_tree("").await.unwrap().data.clone() ;

            let root_inode = self.inodes.lock().unwrap().get(&1).unwrap().clone();
            for it in items{
                println!("root item:{:?}",it);
                self.update_inode(root_inode.clone(),it);
            }
            loop {//BFS to look up all dictionary
                if self.queue.lock().unwrap().is_empty(){
                    break;
                }
                let one_inode = self.queue.lock().unwrap().pop_front().unwrap();
                let mut new_items = Vec::new();
                {
                    let it = self.inodes.lock().unwrap().get(&one_inode).unwrap().clone();
                    let mut ct =it.content_type.lock().unwrap();
                    let path=String::new();
                    if let ContentType::Dictionary(load) = *ct{
                        if !load{
                            *ct = ContentType::Dictionary(true);
                            let path = it.get_path();
                            println!("fetch path :{}",path);
                        }
                        if path.len()>1{
                            let t = fetch_tree(&path.clone()).await;
                            new_items = t.unwrap().data.clone() ;
                        }
                    }

                   
                    let mut pc = it.clone();
                    for newit in new_items {
                        println!("import item :{:?}",newit);
                        self.update_inode(pc.clone(),newit); // Await the update_inode call
                    }
                    
                }
                new_items = Vec::new();
            }
            //queue.clear();
        
    }
    pub fn new() -> Self {
        let mut init = DictionaryStore {
            next_inode: AtomicU64::new(2),
            inodes: Arc::new(Mutex::new(HashMap::new())),
            radix_trie: Arc::new(Mutex::new(radix_trie::Trie::new())),
            queue: Arc::new(Mutex::new(VecDeque::new())),
        };
        let root_item = DicItem{
            inode: 1,
            path_name: GPath::new(),
            content_type: Arc::new(Mutex::new(ContentType::Dictionary(false))),
            children: Mutex::new(HashMap::new()),
            parent: UNKNOW_INODE, //  root dictory has no parent
        };
        init.inodes.lock().unwrap().insert(1, root_item.into());
        init
    }
    fn update_inode(&self,pitem:Arc<DicItem>,item:Item){
        self.next_inode.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        let alloc_inode = self.next_inode.load(std::sync::atomic::Ordering::Relaxed);
        
        assert!(alloc_inode < READONLY_INODE );
        if item.content_type=="directory"{
            self.queue.lock().unwrap().push_back(alloc_inode);
        }
        
        
        let parent = pitem;
        let newitem = Arc::new(DicItem::new(alloc_inode, parent.get_inode(),item));
       
        parent.push_children(newitem.clone());
        self.radix_trie.lock().unwrap().insert(newitem.get_path(), alloc_inode);
        self.inodes.lock().unwrap().insert(alloc_inode, newitem);

    }

    pub fn import(&self){
        let handler  = get_handle();
        // 在阻塞线程中运行异步任务
        let items = futures::executor::block_on(async move {
            handler.spawn(async move {
                fetch_tree("").await.unwrap().data 
            }).await.unwrap()
        });


        
        let root_inode = self.inodes.lock().unwrap().get(&1).unwrap().clone();
        for it in items{
            println!("root item:{:?}",it);
            self.update_inode(root_inode.clone(),it);
        }
        loop {//BFS to look up all dictionary
            if self.queue.lock().unwrap().is_empty(){
                break;
            }
            let one_inode = self.queue.lock().unwrap().pop_front().unwrap();
            let mut new_items = Vec::new();
            {
                let it = self.inodes.lock().unwrap().get(&one_inode).unwrap().clone();
                if let ContentType::Dictionary(load) = *it.content_type.lock().unwrap(){
                    if !load{
                        let path = it.get_path();
                        println!("fetch path :{}",path);
                        let handler  = get_handle();
                        // 在阻塞线程中运行异步任务
                        new_items =futures::executor::block_on(async move {
                            handler.spawn(async move {
                                fetch_tree(&path).await.unwrap().data 
                            }).await.unwrap()
                        });
                
                    }
                   
                }
                let mut pc = it.clone();
                for newit in new_items {
                    println!("import item :{:?}",newit);
                    self.update_inode(pc.clone(),newit); // Await the update_inode call
                }
                let mut content_type = pc.content_type.lock().unwrap();
                *content_type = ContentType::Dictionary(true);
            }
            new_items = Vec::new();
        }
        //queue.clear();
    }
    
    pub fn find_path(&self,inode :u64)-> Option<GPath>{
        self.inodes.lock().unwrap().get(&inode).map(|item| item.path_name.clone())
    }
    pub fn get_inode(&self,inode: u64) -> Result<Arc<DicItem>, io::Error> {
        match self.inodes.lock().unwrap().get(&inode) {
            Some(item) => Ok(item.clone()),
            None=>Err(io::Error::new(io::ErrorKind::NotFound, "inode not found"))
        }
       
    }
    pub fn get_by_path(&self, path: &str) -> Result<Arc<DicItem>, io::Error> {
        let binding = self.radix_trie.lock().unwrap();
        let inode = binding.get(path).ok_or(io::Error::new(io::ErrorKind::NotFound, "path not found"))?;
        self.get_inode(*inode)
    }
    fn find_children(&self,parent: u64) -> Result<DicItem,io::Error>{
        let path = self.inodes.lock().unwrap().get(&parent).map(|item| item.path_name.clone());
        if let Some(parent_path) = path{
            let l  = self.radix_trie.lock().unwrap();
            let pathstr:String =parent_path.name();
            let u = l.subtrie(&pathstr).unwrap();
            let c = u.children();
        }
        todo!()
    }

    pub fn do_readdir(&self,
        ctx: &Context,
        inode: u64,
        handle: u64,
        size: u32,
        offset: u64,
        add_entry: &mut dyn FnMut(DirEntry) -> std::io::Result<usize>,
    ) -> std::io::Result<()> {
         // 1. 获取目录项
         let directory = self.get_inode(inode).unwrap();
        

        // add_entry(DirEntry {
        //     ino: directory.get_inode(),
        //     offset:0,
        //     name:b".",
        //     type_: entry_type_from_mode(directory.get_stat().attr.st_mode).into(),
        // });
        // add_entry(DirEntry {
        //     ino: directory.parent,
        //     offset:1,
        //     name:b"..",
        //     type_: entry_type_from_mode(directory.get_stat().attr.st_mode).into(),
        // });
         // 2. 确保目录项是一个目录
         if let ContentType::Dictionary(_) = directory.get_tyep() {
             // 3. 获取子目录项
             let  children = directory.children.lock().unwrap();
             let mut total_bytes_written = 0;
             let mut current_offset = 0;

             // 4. 遍历子目录项
             for (i, (name, child)) in children.iter().enumerate() {
                
                 if current_offset as u64 >= offset {
                     // 获取每个目录项的 stat 信息
                     let entry = child.get_stat();
                     // 计算目录项的大小
                     let entry_size = name.len() + std::mem::size_of::<u64>(); // name 字节数 + inode 信息的字节数
                     if total_bytes_written + entry_size as u64 > size as u64 {
                         break;
                     }
                     println!("do_readir name:{},child inode:{},type:{:?},mode:{}",name, child.get_inode(),child.get_tyep(),entry_type_from_mode(entry.attr.st_mode));
                     //entry_type_from_mode(entry.attr.st_mode)
                     // 使用回调函数添加目录项
                     let result = add_entry(DirEntry {
                        ino: child.get_inode(),
                        offset: (i+2) as u64,
                        name:name.as_bytes(),
                        type_: entry_type_from_mode(entry.attr.st_mode).into(),
                     });
 
                     match result {
                         Ok(len) => {
                             total_bytes_written += len as u64;
                             current_offset += 1;
                         }
                         Err(e) => return Err(e),
                     }
                 }
             }
 
             // 5. 返回结果
             Ok(())
         } else {
             Err(io::Error::new(io::ErrorKind::NotFound, "Not a directory"))
         }
    }
}
fn entry_type_from_mode(mode: libc::mode_t) -> u8 {
    match mode & libc::S_IFMT {
        libc::S_IFBLK => libc::DT_BLK,
        libc::S_IFCHR => libc::DT_CHR,
        libc::S_IFDIR => libc::DT_DIR,
        libc::S_IFIFO => libc::DT_FIFO,
        libc::S_IFLNK => libc::DT_LNK,
        libc::S_IFREG => libc::DT_REG,
        libc::S_IFSOCK => libc::DT_SOCK,
        _ => libc::DT_UNKNOWN,
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

