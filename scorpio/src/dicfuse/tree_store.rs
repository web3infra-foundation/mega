use crate::util::{config, GPath};
use bincode::{Decode, Encode};
use rfuse3::raw::reply::ReplyEntry;
use rfuse3::FileType;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::io;
use std::io::{Error, ErrorKind};

use super::abi::{default_dic_entry, default_file_entry};
use super::store::ItemExt;

/// inode -> StorageItem{ inode, parent, name, is_dir, children }
pub struct TreeStorage {
    db: Db,
}

#[derive(Serialize, Deserialize, Clone,Encode,Decode)]
pub struct StorageItem {
    inode: u64,
    parent: u64,
    pub name: String,
    is_dir: bool, // True for Directory .
    children: Vec<u64>,
    pub hash: String,
}

impl StorageItem {
    pub fn get_inode(&self) -> u64 {
        self.inode
    }
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
    pub fn get_children(&self) -> Vec<u64> {
        self.children.clone()
    }
    pub fn get_stat(&self) -> ReplyEntry {
        if self.is_dir {
            default_dic_entry(self.inode)
        } else {
            default_file_entry(self.inode)
        }
    }
    pub async fn get_filetype(&self) -> FileType {
        if self.is_dir {
            FileType::Directory
        } else {
            FileType::RegularFile
        }
    }
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_parent(&self) -> u64 {
        self.parent
    }
}
// use toml::Value;
#[allow(unused)]
impl TreeStorage {
    pub fn new_from_db(db: Db) -> Self {
        TreeStorage { db }
    }
    pub fn new() -> io::Result<Self> {
        let store_path = config::store_path();
        let path = format!("{store_path}/path.db");
        let db = sled::open(path)?;
        Ok(TreeStorage { db })
    }
    /// Insert an item and update the parent item's children list.
    pub fn insert_item(&self, inode: u64, parent: u64, item: ItemExt) -> io::Result<()> {
        // create a  StorageItem
        let is_dir = item.item.content_type == "directory";
        let storage_item = StorageItem {
            inode,
            parent,
            name: item.item.name.clone(),
            is_dir,
            children: Vec::new(),
            hash: item.hash,
        };
        let config = bincode::config::standard();
        // Insert an item into db and update the parent item's children list.
        self.db
            .insert(
                inode.to_be_bytes(),
                bincode::encode_to_vec(&storage_item,config).map_err(Error::other)?,
            )
            .map_err(Error::other)?;

        if parent != 0 {
            let mut parent_item: StorageItem = self.get_storage_item(parent)?;
            //Append the children inode.
            parent_item.children.push(inode);
            // write back
            self.db
                .insert(
                    parent.to_be_bytes(),
                    bincode::encode_to_vec(&parent_item,config).map_err(Error::other)?,
                )
                .map_err(Error::other)?;
        }

        Ok(())
    }

    /// Get a dic item.
    pub fn get_item(&self, inode: u64) -> io::Result<StorageItem> {
        self.get_storage_item(inode)
    }

    /// Delete an item and recursively delete its sub-items.
    pub fn remove_item(&self, inode: u64) -> std::io::Result<()> {
        if let Ok(storage_item) = self.get_storage_item(inode) {
            // Recursively delete child items.
            for child_inode in storage_item.children {
                self.remove_item(child_inode)?;
            }

            // Remove from the parent item's children list.
            if storage_item.parent != 0 {
                let mut parent_item: StorageItem = self.get_storage_item(storage_item.parent)?;
                parent_item.children.retain(|&x| x != inode);
                let config = bincode::config::standard();

                self.db
                    .insert(
                        storage_item.parent.to_be_bytes(),
                       bincode::encode_to_vec(&parent_item, config).map_err(Error::other)?,
                    )
                    .map_err(Error::other)?;
            }

            // Delete current item.
            self.db.remove(inode.to_be_bytes())?;
        } else {
            return Err(Error::new(ErrorKind::NotFound, "Item not found"));
        }
        Ok(())
    }
    pub fn append_child(&self, parent: u64, inode: u64) -> io::Result<()> {
        let mut st = self.get_storage_item(parent)?;
        st.children.push(inode);
        let config = bincode::config::standard();
        self.db
            .insert(
                parent.to_be_bytes(),
                bincode::encode_to_vec(&st, config).map_err(Error::other)?,
            )
            .map_err(Error::other)?;
        Ok(())
    }
    pub fn get_children(&self, inode: u64) -> io::Result<Vec<StorageItem>> {
        let mut children = Vec::new();
        let inode = self.get_storage_item(inode)?;
        for child in inode.children {
            match self.get_item(child) {
                Ok(item) => children.push(item),
                Err(e) => return Err(e),
            }
        }
        Ok(children)
    }
    /// Get StorageItem
    pub fn get_storage_item(&self, inode: u64) -> io::Result<StorageItem> {
        match self.db.get(inode.to_be_bytes())? {
            Some(value) => {
                let config = bincode::config::standard();
                let (item ,_) = bincode::decode_from_slice(&value,config).map_err(Error::other)?;
                Ok(item)
            }
            None => Err(Error::new(ErrorKind::NotFound, "Item not found")),
        }
    }
    pub fn get_all_path(&self, inode: u64) -> io::Result<GPath> {
        let mut names = Vec::new();
        let mut ino = inode;
        while ino != 1 {
            let temp_item = self.get_item(ino).unwrap();
            names.push(temp_item.name);
            ino = temp_item.parent;
        }
        names.reverse(); // reverse the names to get the all path of this item.
        Ok(GPath { path: names })
    }
    /// when the dir's hash changes,we need to update the hash value in the db.
    pub fn update_item_hash(&self, inode: u64, hash: String) -> io::Result<()> {
        let mut item = self.get_storage_item(inode)?;
        item.hash = hash;
        let config = bincode::config::standard();
        self.db
            .insert(
                inode.to_be_bytes(),
                bincode::encode_to_vec(&item,config).map_err(Error::other)?,
            )
            .map_err(Error::other)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicfuse::store::Item;
    use core::panic;

    fn setup(path: &str) -> io::Result<TreeStorage> {
        if std::path::Path::new(path).exists() {
            std::fs::remove_dir_all(path).ok();
        }
        std::fs::create_dir_all(path).ok();
        let db = sled::open(path)?;
        Ok(TreeStorage::new_from_db(db))
    }
    fn unset(path: &str) {
        std::fs::remove_dir_all(path).ok();
    }
    #[test]
    fn test_insert_and_get_item() {
        let storage = setup("test_insert_and_get_item").unwrap();
        let item = ItemExt {
            item: Item {
                name: String::from("Test Item"),
                path: String::from("/path/to/item"),
                content_type: String::from("text/plain"),
            },
            hash: String::new(),
        };
        storage.insert_item(1, 0, item.clone()).unwrap();
        let retrieved_item = storage.get_item(1).unwrap();
        assert_eq!(item.item.name, retrieved_item.name);
        unset("test_insert_and_get_item");
    }

    #[test]
    fn test_remove_item() {
        let storage = setup("test_remove_item").unwrap();
        let item = ItemExt {
            item: Item {
                name: String::from("Test Item"),
                path: String::from("/path/to/item"),
                content_type: String::from("text/plain"),
            },
            hash: String::new(),
        };
        storage.insert_item(2, 0, item.clone()).unwrap();
        storage.remove_item(2).unwrap();
        unset("test_remove_item");
    }

    #[test]
    fn test_list_items() {
        let storage = setup("test_list_items").unwrap();
        let item1 = ItemExt {
            item: Item {
                name: String::from("Test Item 1"),
                path: String::from("/path/to/item1"),
                content_type: String::from("text/plain"),
            },
            hash: String::new(),
        };
        let item2 = ItemExt {
            item: Item {
                name: String::from("Test Item 2"),
                path: String::from("/path/to/item2"),
                content_type: String::from("image/png"),
            },
            hash: String::new(),
        };
        storage.insert_item(3, 0, item1.clone()).unwrap();
        storage.insert_item(4, 0, item2.clone()).unwrap();

        unset("test_list_items");
    }

    #[test]
    fn test_get_nonexistent_item() {
        let storage = setup("test_get_nonexistent_item").unwrap();
        let result = storage.get_item(999);
        assert!(result.is_err());
        unset("test_get_nonexistent_item");
    }

    #[test]
    fn test_remove_nonexistent_item() {
        let storage = setup("test_remove_nonexistent_item").unwrap();
        let result = storage.remove_item(999);
        if result.is_ok() {
            panic!("should error");
        }
        unset("test_remove_nonexistent_item");
    }

    #[test]
    fn test_traverse_directory_structure() {
        let storage = setup("/tmp/test_traverse_directory_structure").unwrap();
        println!("test begin...");
        // Function to traverse and collect directory structure
        fn traverse(storage: &TreeStorage, inode: u64, depth: usize) {
            if let Ok(item) = storage.get_item(inode) {
                for child_inode in item.children {
                    if item.is_dir {
                        println!("{}Directory: {}", "  ".repeat(depth), item.name);
                        traverse(storage, child_inode, depth + 1);
                    } else {
                        println!("{}File: {}", "  ".repeat(depth), item.name);
                    }
                }
            }
        }

        // Start traversal from root (inode 1)
        traverse(&storage, 1, 0);

        unset("/tmp/test_traverse_directory_structure");
    }
}
