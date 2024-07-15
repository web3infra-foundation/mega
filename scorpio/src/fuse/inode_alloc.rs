use std::{collections::HashMap, sync::{atomic::{AtomicU64, Ordering}, Mutex}};

 
#[allow(unused)]
const VFS_MAX_INO: u64 = 0xff_ffff_ffff_ffff;
#[allow(unused)]
const READONLY_INODE :u64 = 0xffff_ffff;
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
    pub fn alloc_inode(&mut self,ovl_inode:u64)-> u64{
        self.next_ino_batch.fetch_add(1, Ordering::Relaxed);
        let ainode =  self.next_ino_batch.load(Ordering::Acquire);
        let mut alloc = self.alloc.lock().unwrap();
        alloc.insert(ainode,ovl_inode);
        ainode
    }
    pub fn get_ovl_inode(&self,path_inode:u64)-> Option<u64>{
        self.alloc.lock().unwrap().get(&path_inode).copied()
    }
}