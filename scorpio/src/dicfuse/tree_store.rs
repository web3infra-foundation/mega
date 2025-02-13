
use sled::Db;
use serde::{Serialize, Deserialize};
use std::io::{Error, ErrorKind};
use std::io;
use super::store::Item;
#[allow(unused)]
pub struct TreeStorage {
    db: Db,
}
const CONFIG_PATH: &str = "config.toml";
#[derive(Serialize,Deserialize)]
struct StorageItem{
    inode: u64,
    parent : u64,
    name : String ,
    is_dir: bool , // True for Directory . 
    children:Vec<u64>
}
use toml::Value;
#[allow(unused)]
impl TreeStorage {
    pub fn new_from_db(db: Db) -> Self {
        TreeStorage { db }
    }
    pub fn new() -> io::Result<Self>{
        let config_content = std::fs::read_to_string(CONFIG_PATH)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        let config: Value = toml::de::from_str(&config_content)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        let store_path = config["store_path"]
            .as_str()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "store_path not found in config"))?;

        let path = format!("{}/path.db", store_path);
        let db = sled::open(path).unwrap();
        Ok(TreeStorage { db })
    }
    /// Insert an item and update the parent item's children list.
    pub fn insert_item(&self, inode: u64, parent: u64, item: Item) -> io::Result<()> {
        // create a  StorageItem
        let is_dir = item.content_type == "directory" ;
        let storage_item = StorageItem {
            inode,
            parent,
            name: item.name.clone(),
            is_dir,
            children: Vec::new(),
        };

        // Insert an item into db and update the parent item's children list.
        self.db
            .insert(inode.to_be_bytes(), bincode::serialize(&storage_item).map_err(|e| Error::new(ErrorKind::Other, e))?)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        if parent != 0 {
            let mut parent_item: StorageItem = self.get_storage_item(parent)?
                .ok_or_else(|| Error::new(ErrorKind::NotFound, "Parent not found"))?;
            parent_item.children.push(inode);
            self.db
                .insert(parent.to_be_bytes(), bincode::serialize(&parent_item).map_err(|e| Error::new(ErrorKind::Other, e))?)
                .map_err(|e| Error::new(ErrorKind::Other, e))?;
        }

        Ok(())
    }

    /// Get a dic item.
    pub fn get_item(&self, inode: u64) -> Option<Item> {

        match self.get_storage_item(inode){
            Ok(storage_item) => {
                if let Some(storage_item) = storage_item{
                    let item = Item {
                        name: storage_item.name,
                        path: String::new(), 
                        content_type: if storage_item.is_dir {
                            "directory".to_string()
                        } else {
                            "file".to_string()
                        },
                    };
                    Some(item)
                }else {
                    None
                }
            },
            Err(_) =>{
                None
            },
        }
       
    }

    /// Delete an item and recursively delete its sub-items.
    pub fn remove_item(&self, inode: u64) -> std::io::Result<()> {
        if let Some(storage_item) = self.get_storage_item(inode)? {
            // Recursively delete child items.
            for child_inode in storage_item.children {
                self.remove_item(child_inode)?;
            }

            // Remove from the parent item's children list.
            if storage_item.parent != 0 {
                let mut parent_item: StorageItem = self.get_storage_item(storage_item.parent)?
                    .ok_or_else(|| Error::new(ErrorKind::NotFound, "Parent not found"))?;
                parent_item.children.retain(|&x| x != inode);
                self.db
                    .insert(storage_item.parent.to_be_bytes(), bincode::serialize(&parent_item).map_err(|e| Error::new(ErrorKind::Other, e))?)
                    .map_err(|e| Error::new(ErrorKind::Other, e))?;
            }

            // Delete current item.
            self.db.remove(inode.to_be_bytes())?;
        }else {
            return Err(Error::new(ErrorKind::NotFound, "Item not found"));
        }
        Ok(())
    }

    /// Get StorageItem
    fn get_storage_item(&self, inode: u64) -> io::Result<Option<StorageItem>> {
        match self.db.get(inode.to_be_bytes())? {
            Some(value) => {
                let item: StorageItem = bincode::deserialize(&value)
                    .map_err(|e| Error::new(ErrorKind::Other, e))?;
                Ok(Some(item))
            }
            None => Ok(None),
        }
    }
}
#[cfg(test)]
mod tests {
    use core::panic;


    use super::*;

    fn setup(path:&str ) -> TreeStorage {
        let db = sled::open(path).unwrap();
        TreeStorage::new_from_db(db)
    }
    fn unset(path:&str ){
        std::fs::remove_dir_all(path).ok();
    }
    #[test]
    fn test_insert_and_get_item() {
        let storage = setup("test_insert_and_get_item");
        let item = Item {
            name: String::from("Test Item"),
            path: String::from("/path/to/item"),
            content_type: String::from("text/plain"),
        };
        storage.insert_item(1,0,item.clone()).unwrap();
        let retrieved_item = storage.get_item(1).unwrap();
        assert_eq!(item.name, retrieved_item.name);
        unset("test_insert_and_get_item");
    }

    #[test]
    fn test_remove_item() {
        let storage = setup("test_remove_item");
        let item = Item {
            name: String::from("Test Item"),
            path: String::from("/path/to/item"),
            content_type: String::from("text/plain"),
        };
        storage.insert_item(2,0,item.clone()).unwrap();
        storage.remove_item(2).unwrap();
        unset("test_remove_item");
    }

    #[test]
    fn test_list_items() {
        let storage = setup("test_list_items");
        let item1 = Item {
            name: String::from("Test Item 1"),
            path: String::from("/path/to/item1"),
            content_type: String::from("text/plain"),
        };
        let item2 = Item {
            name: String::from("Test Item 2"),
            path: String::from("/path/to/item2"),
            content_type: String::from("image/png"),
        };
        storage.insert_item(3,0,item1.clone()).unwrap();
        storage.insert_item(4,0,item2.clone()).unwrap();

        unset("test_list_items");
    }

    #[test]
    fn test_get_nonexistent_item() {
        let storage = setup("test_get_nonexistent_item");
        let result = storage.get_item(999);
        assert_eq!(result,None);
        unset("test_get_nonexistent_item");
    }

    #[test]
    fn test_remove_nonexistent_item() {
        let storage = setup("test_remove_nonexistent_item");
        let result = storage.remove_item(999);
        if result.is_ok(){
            panic!("should error");
        }
        unset("test_remove_nonexistent_item");
    }

    #[test]
    fn test_traverse_directory_structure() {
        let storage = setup("/home/luxian/megadir/store/path.db");
        println!("test begin...");
        // Function to traverse and collect directory structure
        fn traverse(storage: &TreeStorage, inode: u64, depth: usize) {
            if let Some(item) = storage.get_item(inode) {
                if item.content_type == "directory" {
                    println!("{}Directory: {}", "  ".repeat(depth), item.name);
                    if let Some(storage_item) = storage.get_storage_item(inode).unwrap() {
                        for child_inode in storage_item.children {
                            traverse(storage, child_inode, depth + 1);
                        }
                    }
                } else {
                    println!("{}File: {}", "  ".repeat(depth), item.name);
                }
            }
        }

        // Start traversal from root (inode 1)
        traverse(&storage, 1, 0);

        //unset("/home/luxian/megadir/store/path.db");
    }
}

