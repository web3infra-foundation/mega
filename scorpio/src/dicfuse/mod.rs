mod store;
mod abi;
mod async_io;
mod tree_store;
use std::{collections::HashMap, ffi::{OsStr, OsString}, sync::Arc};
use crate::manager::fetch::fetch_tree;
use crate::util::GPath;
use mercury::internal::object::tree::TreeItemMode;
use reqwest::Client;

use fuse3::raw::reply::ReplyEntry;
use store::{DicItem, DictionaryStore};
pub struct Dicfuse{
    pub store: Arc<DictionaryStore>,
    open_buff: Arc<tokio::sync::RwLock<HashMap<u64, Vec<u8>>>>,
}
#[allow(unused)]
impl Dicfuse{
    pub async fn new() -> Self {
        Self {
            store: DictionaryStore::new().await.into(), // Assuming DictionaryStore has a new() method
            open_buff: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
    pub async fn get_stat(&self,item:Arc<DicItem>) -> ReplyEntry {
        let mut e  =item.get_stat().await;
        let rl = self.open_buff.read().await;
        if let Some(datas) = rl.get(&item.get_inode()){
            e.attr.size = datas.len() as u64;
        }
        e
    }
    async fn load_one_file(&self, parent: u64, name: &OsStr) -> std::io::Result<()>{
        let parent_item = self.store.get_inode(parent).await?;
        let tree = fetch_tree(&GPath::from(parent_item.get_path())).await.unwrap();
       
        let client = Client::new();
        for i in tree.tree_items{
            let name_os = OsString::from(&i.name);
            if name_os!=name{
                continue;
            }else if i.mode!=TreeItemMode::Blob{
                return Ok(());
            }

            let url = format!("http://localhost:8000/api/v1/file/blob/{}",i.id);//TODO: configabel.
            // Send GET request
            let response = client.get(url).send().await.unwrap();//todo error 
            
            // Ensure that the response status is successful
            if response.status().is_success() {
                // Get the binary data from the response body
                let content = response.bytes().await.unwrap();//TODO error
                
                // Store the content in a Vec<u8>
                let data: Vec<u8> = content.to_vec();
                //let child_osstr = OsStr::new(&i.name);
                let c_lock =parent_item.children.lock().await;
                let child = c_lock.get(&i.name).unwrap();

                self.open_buff.write().await.insert(child.get_inode(), data);
                
            } else {
                eprintln!("Request failed with status: {}", response.status());
            }
            break;
            
        }
        Ok(())
    }
    pub async fn load_fiels(&self, parent_item :Arc<DicItem>){
        let tree = fetch_tree(&GPath::from(parent_item.get_path())).await.unwrap();
        
        let mut is_first  = true;
        let client = Client::new();
        for i in tree.tree_items{
            if i.mode!=TreeItemMode::Blob{
                continue;
            }
            let url = format!("http://localhost:8000/api/v1/file/blob/{}",i.id);//TODO: configabel.
            // Send GET request
            let response = client.get(url).send().await.unwrap();//todo error 
            
            // Ensure that the response status is successful
            if response.status().is_success() {
                // Get the binary data from the response body
                let content = response.bytes().await.unwrap();//TODO error
                
                // Store the content in a Vec<u8>
                let data: Vec<u8> = content.to_vec();
                //let child_osstr = OsStr::new(&i.name);
                let c_lock =parent_item.children.lock().await;
                let child = c_lock.get(&i.name).unwrap();
                if is_first{
                    match self.open_buff.write().await.get(&child.get_inode()) {
                        Some(_) => {
                            break;
                            // is loaded , no need to reload ;
                        },
                        None => {
                            // this dictionary is not loaded ,  just go ahead.
                            is_first = false;
                        },
                    }
                }
                self.open_buff.write().await.insert(child.get_inode(), data);
                
            } else {
                eprintln!("Request failed with status: {}", response.status());
            }
            
        }
    }


}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use tokio::signal;

    use crate::dicfuse::Dicfuse;


    #[tokio::test]
    async fn test_mount_dic(){
        let fs = Dicfuse::new().await;
        let mountpoint =OsStr::new("/home/luxian/dic") ;
        let mut mount_handle =  crate::server::mount_filesystem(fs, mountpoint).await;
        let handle = &mut mount_handle;
        tokio::select! {
            res = handle => res.unwrap(),
            _ = signal::ctrl_c() => {
                mount_handle.unmount().await.unwrap()
            }
        }
    
    }
    
}
