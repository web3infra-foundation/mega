
use sled::Db;
use serde::{Serialize, Deserialize};

use super::store::Item;
#[allow(unused)]
pub struct TreeStorage {
    db: Db,
}

#[derive(Serialize,Deserialize)]
struct StorageItem{
    inode: u64,
    parent : u64,
    name : String ,
    is_dir: bool , // Ture for Dictionary . 
    childrem:Vec<u64>
}
#[allow(unused)]
impl TreeStorage {
    pub fn new(db: Db) -> Self {
        TreeStorage { db }
    }
    pub fn insert_item(&self, inode: u64,item: Item) -> std::io::Result<()> {
        //let key = self.db.generate_id()?; // Automatically generate an inode number
        let value = bincode::serialize(&item).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        self.db.insert(inode.to_be_bytes(), value).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(())
    }
    pub fn get_item(&self, inode: u64) -> std::io::Result<Option<Item>> {
        if let Some(value) = self.db.get(inode.to_be_bytes())? {
            let item: Item = bincode::deserialize(&value).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }
    pub fn remove_item(&self, inode: u64) -> std::io::Result<Option<Item>> {
        if let Some(value) = self.db.remove(inode.to_be_bytes())? {
            let item: Item = bincode::deserialize(&value).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }
    pub fn list_items(&self) -> std::io::Result<Vec<Item>> {
        let mut items = Vec::new();
        for result in self.db.iter() {
            let (_, value) = result?;
            let item: Item = bincode::deserialize(&value).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            items.push(item);
        }
        Ok(items)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn setup(path:&str ) -> TreeStorage {
        let db = sled::open(path).unwrap();
        TreeStorage::new(db)
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
        storage.insert_item(1,item.clone()).unwrap();
        let retrieved_item = storage.get_item(1).unwrap().unwrap();
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
        storage.insert_item(2,item.clone()).unwrap();
        let removed_item = storage.remove_item(2).unwrap().unwrap();
        assert_eq!(item.path, removed_item.path);
        assert!(storage.get_item(2).unwrap().is_none());
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
        storage.insert_item(3,item1.clone()).unwrap();
        storage.insert_item(4,item2.clone()).unwrap();
        let items = storage.list_items().unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.contains(&item1));
        assert!(items.contains(&item2));
        unset("test_list_items");
    }

    #[test]
    fn test_get_nonexistent_item() {
        let storage = setup("test_get_nonexistent_item");
        let result = storage.get_item(999).unwrap();
        assert!(result.is_none());
        unset("test_get_nonexistent_item");
    }

    #[test]
    fn test_remove_nonexistent_item() {
        let storage = setup("test_remove_nonexistent_item");
        let result = storage.remove_item(999).unwrap();
        assert!(result.is_none());
        unset("test_remove_nonexistent_item");
    }
}

