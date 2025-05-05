use std::sync::atomic::AtomicU64;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

// Alloc inode numbers at one batch
#[allow(unused)]
const INODE_ALLOC_BATCH: u64 = 0x1_0000_0000;

pub struct InodeAlloc {
    next_ino_batch: AtomicU64,
    // Alloc inode/INODE_ALLOC_BATCH  -->  Ovlay-Inode
    alloc: Arc<Mutex<HashMap<u64, u64>>>,
}
// Note:
// AtomicU64 uses atomic hardware instructions to ensure thread-safe access.
// Wrapping it in an Arc would be redundant unless you specifically need to share
// the same atomic counter across multiple InodeAlloc instances.
impl Clone for InodeAlloc {
    fn clone(&self) -> Self {
        InodeAlloc {
            next_ino_batch: AtomicU64::new(
                self.next_ino_batch
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            alloc: self.alloc.clone(),
        }
    }
}

#[allow(unused)]
impl InodeAlloc {
    pub fn new() -> Self {
        InodeAlloc {
            next_ino_batch: AtomicU64::new(1),
            alloc: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub async fn alloc_inode(&self, ovl_inode: u64) -> u64 {
        self.next_ino_batch
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let ainode = self
            .next_ino_batch
            .load(std::sync::atomic::Ordering::Relaxed);
        let mut alloc = self.alloc.lock().await;
        alloc.insert(ainode, ovl_inode);
        ainode
    }
    pub async fn get_ovl_inode(&self, inode_batch: u64) -> Option<u64> {
        self.alloc.lock().await.get(&inode_batch).copied()
    }
    pub async fn clear(&self) {
        self.alloc.lock().await.clear();
        self.next_ino_batch
            .store(1, std::sync::atomic::Ordering::Relaxed);
    }
}
