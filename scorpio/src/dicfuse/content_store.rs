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
}
