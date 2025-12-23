use crate::util::config;
use sled::Db;
use std::io;
use std::io::{Error, ErrorKind};

pub struct ContentStorage {
    db: Db,
}
#[allow(unused)]
impl ContentStorage {
    pub fn new_from_db(db: Db) -> Self {
        ContentStorage { db }
    }
    pub fn new() -> io::Result<Self> {
        let store_path = config::store_path();
        Self::new_with_path(store_path)
    }

    pub fn new_with_path(store_path: &str) -> io::Result<Self> {
        let path = format!("{store_path}/content.db");
        let db = sled::open(path)?;
        Ok(ContentStorage { db })
    }
    pub fn insert_file(&self, inode: u64, content: &[u8]) -> io::Result<()> {
        self.db
            .insert(inode.to_be_bytes(), content)
            .map_err(Error::other)?;
        Ok(())
    }

    pub fn get_file_content(&self, inode: u64) -> io::Result<Vec<u8>> {
        match self.db.get(inode.to_be_bytes())? {
            Some(value) => Ok(value.to_vec()),
            None => Err(Error::new(ErrorKind::NotFound, "File not found")),
        }
    }
    pub fn remove_file(&self, inode: u64) -> std::io::Result<()> {
        self.db.remove(inode.to_be_bytes())?;
        Ok(())
    }

    /// Clear all persisted file contents.
    ///
    /// This is used to recover from partially-initialized stores (e.g., when a previous import
    /// was interrupted) and to avoid reusing stale cached contents.
    pub fn clear_all(&self) -> io::Result<()> {
        self.db.clear().map_err(Error::other)?;
        Ok(())
    }
}
