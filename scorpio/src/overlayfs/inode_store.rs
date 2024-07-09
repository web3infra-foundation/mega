// Copyright (C) 2023 Ant Group. All rights reserved.
//  2024 From [fuse_backend_rs](https://github.com/cloud-hypervisor/fuse-backend-rs) 
// SPDX-License-Identifier: Apache-2.0

use std::io::{Error, ErrorKind, Result};
use std::{
    collections::HashMap,
    sync::{atomic::Ordering, Arc},
};

use super::{Inode, OverlayInode, VFS_MAX_INO};

use radix_trie::Trie;

pub struct InodeStore {
    // Active inodes.
    inodes: HashMap<Inode, Arc<OverlayInode>>,
    // Deleted inodes which were unlinked but have non zero lookup count.
    deleted: HashMap<Inode, Arc<OverlayInode>>,
    // Path to inode mapping, used to reserve inode number for same path.
    path_mapping: Trie<String, Inode>,
    next_inode: u64,
}
#[allow(unused)]
impl InodeStore {

    pub(crate) fn new() -> Self {
        Self {
            inodes: HashMap::new(),
            deleted: HashMap::new(),
            path_mapping: Trie::new(),
            next_inode: 1,
        }
    }

    pub(crate) fn alloc_unique_inode(&mut self) -> Result<Inode> {
        // Iter VFS_MAX_INO times to find a free inode number.
        let mut ino = self.next_inode;
        for _ in 0..VFS_MAX_INO {
            if ino > VFS_MAX_INO {
                ino = 1;
            }
            if !self.inodes.contains_key(&ino) && !self.deleted.contains_key(&ino) {
                self.next_inode = ino + 1;
                return Ok(ino);
            }
            ino += 1;
        }
        error!("reached maximum inode number: {}", VFS_MAX_INO);
        Err(Error::new(
            ErrorKind::Other,
            format!("maximum inode number {} reached", VFS_MAX_INO),
        ))
    }

    pub(crate) fn alloc_inode(&mut self, path: &String) -> Result<Inode> {
        match self.path_mapping.get(path) {
            // If the path is already in the mapping, return the reserved inode number.
            Some(v) => Ok(*v),
            // Or allocate a new inode number.
            None => self.alloc_unique_inode(),
        }
    }

    pub(crate) fn insert_inode(&mut self, inode: Inode, node: Arc<OverlayInode>) {
        self.path_mapping.insert(node.path.clone(), inode);
        self.inodes.insert(inode, node);
    }

    pub(crate) fn get_inode(&self, inode: Inode) -> Option<Arc<OverlayInode>> {
        self.inodes.get(&inode).cloned()
    }

    pub(crate) fn get_deleted_inode(&self, inode: Inode) -> Option<Arc<OverlayInode>> {
        self.deleted.get(&inode).cloned()
    }

    // Return the inode only if it's permanently deleted from both self.inodes and self.deleted_inodes.
    pub(crate) fn remove_inode(
        &mut self,
        inode: Inode,
        path_removed: Option<String>,
    ) -> Option<Arc<OverlayInode>> {
        let removed = match self.inodes.remove(&inode) {
            Some(v) => {
                // Refcount is not 0, we have to delay the removal.
                if v.lookups.load(Ordering::Relaxed) > 0 {
                    self.deleted.insert(inode, v.clone());
                    return None;
                }
                Some(v)
            }
            None => {
                // If the inode is not in hash, it must be in deleted_inodes.
                match self.deleted.get(&inode) {
                    Some(v) => {
                        // Refcount is 0, the inode can be removed now.
                        if v.lookups.load(Ordering::Relaxed) == 0 {
                            self.deleted.remove(&inode)
                        } else {
                            // Refcount is not 0, the inode will be removed later.
                            None
                        }
                    }
                    None => None,
                }
            }
        };

        if let Some(path) = path_removed {
            self.path_mapping.remove(&path);
        }
        removed
    }

    // As a debug function, print all inode numbers in hash table.
    // This function consumes quite lots of memory, so it's disabled by default.
    #[allow(dead_code)]
    pub(crate) fn debug_print_all_inodes(&self) {
        // Convert the HashMap to Vector<(inode, pathname)>
        let mut all_inodes = self
            .inodes
            .iter()
            .map(|(inode, ovi)| (inode, ovi.path.clone(), ovi.lookups.load(Ordering::Relaxed)))
            .collect::<Vec<_>>();
        all_inodes.sort_by(|a, b| a.0.cmp(b.0));
        trace!("all active inodes: {:?}", all_inodes);

        let mut to_delete = self
            .deleted
            .iter()
            .map(|(inode, ovi)| (inode, ovi.path.clone(), ovi.lookups.load(Ordering::Relaxed)))
            .collect::<Vec<_>>();
        to_delete.sort_by(|a, b| a.0.cmp(b.0));
        trace!("all deleted inodes: {:?}", to_delete);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_alloc_unique() {
        let mut store = InodeStore::new();
        let empty_node = Arc::new(OverlayInode::new());
        store.insert_inode(1, empty_node.clone());
        store.insert_inode(2, empty_node.clone());
        store.insert_inode(VFS_MAX_INO - 1, empty_node.clone());

        let inode = store.alloc_unique_inode().unwrap();
        assert_eq!(inode, 3);
        assert_eq!(store.next_inode, 4);

        store.next_inode = VFS_MAX_INO - 1;
        let inode = store.alloc_unique_inode().unwrap();
        assert_eq!(inode, VFS_MAX_INO);

        let inode = store.alloc_unique_inode().unwrap();
        assert_eq!(inode, 3);
    }

    #[test]
    fn test_alloc_existing_path() {
        let mut store = InodeStore::new();
        let mut node_a = OverlayInode::new();
        node_a.path = "/a".to_string();
        store.insert_inode(1, Arc::new(node_a));
        let mut node_b = OverlayInode::new();
        node_b.path = "/b".to_string();
        store.insert_inode(2, Arc::new(node_b));
        let mut node_c = OverlayInode::new();
        node_c.path = "/c".to_string();
        store.insert_inode(VFS_MAX_INO - 1, Arc::new(node_c));

        let inode = store.alloc_inode(&"/a".to_string()).unwrap();
        assert_eq!(inode, 1);

        let inode = store.alloc_inode(&"/b".to_string()).unwrap();
        assert_eq!(inode, 2);

        let inode = store.alloc_inode(&"/c".to_string()).unwrap();
        assert_eq!(inode, VFS_MAX_INO - 1);

        let inode = store.alloc_inode(&"/notexist".to_string()).unwrap();
        assert_eq!(inode, 3);
    }

    #[test]
    fn test_remove_inode() {
        let mut store = InodeStore::new();
        let mut node_a = OverlayInode::new();
        node_a.lookups.fetch_add(1, Ordering::Relaxed);
        node_a.path = "/a".to_string();
        store.insert_inode(1, Arc::new(node_a));

        let mut node_b = OverlayInode::new();
        node_b.path = "/b".to_string();
        store.insert_inode(2, Arc::new(node_b));

        let mut node_c = OverlayInode::new();
        node_c.lookups.fetch_add(1, Ordering::Relaxed);
        node_c.path = "/c".to_string();
        store.insert_inode(VFS_MAX_INO - 1, Arc::new(node_c));

        let inode = store.alloc_inode(&"/new".to_string()).unwrap();
        assert_eq!(inode, 3);

        // Not existing.
        let inode = store.remove_inode(4, None);
        assert!(inode.is_none());

        // Existing but with non-zero refcount.
        let inode = store.remove_inode(1, None);
        assert!(inode.is_none());
        assert!(store.get_deleted_inode(1).is_some());
        assert!(store.path_mapping.get(&"/a".to_string()).is_some());

        // Remove again with file path.
        let inode = store.remove_inode(1, Some("/a".to_string()));
        assert!(inode.is_none());
        assert!(store.get_deleted_inode(1).is_some());
        assert!(store.path_mapping.get(&"/a".to_string()).is_none());

        // Node b has refcount 0, removing will be permanent.
        let inode = store.remove_inode(2, Some("/b".to_string()));
        assert!(inode.is_some());
        assert!(store.get_deleted_inode(2).is_none());
        assert!(store.path_mapping.get(&"/b".to_string()).is_none());

        // Allocate new inode, it should reuse inode 2 since inode 1 is still in deleted list.
        store.next_inode = 1;
        let inode = store.alloc_inode(&"/b".to_string()).unwrap();
        assert_eq!(inode, 2);

        // Allocate inode with path "/c" will reuse its inode number.
        let inode = store.alloc_inode(&"/c".to_string()).unwrap();
        assert_eq!(inode, VFS_MAX_INO - 1);
    }
}
