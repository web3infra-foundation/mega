mod store;
mod abi;
mod async_io;
mod tree_store;
use std::{collections::HashMap,sync::Arc};


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
    // pub async fn pull_fiel(&self,parent_inode:u64)->Result<()>{
    //     let parent_item = self.store.get_inode(parent_inode).await?;
    //     let tree = fetch_tree(&GPath::from(parent_item.get_path())).await.unwrap();

    //     Ok(())
    // }
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
