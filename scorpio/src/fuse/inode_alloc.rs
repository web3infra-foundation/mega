use std::{collections::HashMap, sync::{atomic::{AtomicU64, Ordering}, Mutex}};

 
#[allow(unused)]

// Alloc inode numbers at one batch
#[allow(unused)]
const INODE_ALLOC_BATCH:u64 = 0x1_0000_0000;
#[allow(unused)]
pub struct InodeAlloc {
    next_ino_batch: AtomicU64,
    // Alloc inode/INODE_ALLOC_BATCH  -->  Ovlay-Inode
    alloc: Mutex<HashMap<u64,u64>> ,
}

impl InodeAlloc{
    pub fn new()-> Self{
        InodeAlloc{
            next_ino_batch: AtomicU64::new(1),
            alloc: Mutex::new(HashMap::new())
        }
    }
    pub fn alloc_inode(&self,ovl_inode:u64)-> u64{
        self.next_ino_batch.fetch_add(1, Ordering::Relaxed);
        let ainode =  self.next_ino_batch.load(Ordering::Acquire);
        let mut alloc = self.alloc.lock().unwrap();
        alloc.insert(ainode,ovl_inode);
        ainode
    }
    pub fn get_ovl_inode(&self,inode_batch:u64)-> Option<u64>{
        self.alloc.lock().unwrap().get(&inode_batch).copied()
    }

}