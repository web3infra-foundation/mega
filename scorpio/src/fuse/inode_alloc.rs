use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::util::atomic::AtomicU64;

 


// Alloc inode numbers at one batch
#[allow(unused)]
const INODE_ALLOC_BATCH:u64 = 0x1_0000_0000;

#[derive(Clone)]
pub struct InodeAlloc {
    next_ino_batch: AtomicU64,
    // Alloc inode/INODE_ALLOC_BATCH  -->  Ovlay-Inode
    alloc: Arc<Mutex<HashMap<u64,u64>> >,
}

#[allow(unused)]
impl InodeAlloc{
    pub fn new()-> Self{
        InodeAlloc{
            next_ino_batch: AtomicU64::new(1),
            alloc: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub async fn alloc_inode(&self,ovl_inode:u64)-> u64{
        self.next_ino_batch.fetch_add(1).await;
        let ainode =  self.next_ino_batch.load().await;
        let mut alloc = self.alloc.lock().await;
        alloc.insert(ainode,ovl_inode);
        ainode
    }
    pub async fn get_ovl_inode(&self,inode_batch:u64)-> Option<u64>{
        self.alloc.lock().await.get(&inode_batch).copied()
    }
    pub async fn clear(&self){
        self.alloc.lock().await.clear();
        self.next_ino_batch.store(1).await;
    }

}