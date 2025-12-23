use crate::util::config;
use sled::Db;
use std::io;
use std::io::Error;

/// Persistent file-size store (inode -> size in bytes).
///
/// This is used to report correct `st_size` for files even when file contents are fetched lazily.
/// Without this, cold starts may report size=0 for all files, which can cause some callers to
/// treat files as EOF and never trigger `read()` (and thus never trigger lazy blob fetch).
pub struct SizeStorage {
    db: Db,
}

#[allow(unused)]
impl SizeStorage {
    pub fn new() -> io::Result<Self> {
        let store_path = config::store_path();
        Self::new_with_path(store_path)
    }

    pub fn new_with_path(store_path: &str) -> io::Result<Self> {
        let path = format!("{store_path}/size.db");
        let db = sled::open(path)?;
        Ok(SizeStorage { db })
    }

    pub fn set_size(&self, inode: u64, size: u64) -> io::Result<()> {
        self.db
            .insert(inode.to_be_bytes(), size.to_be_bytes().to_vec())
            .map_err(Error::other)?;
        Ok(())
    }

    pub fn get_size(&self, inode: u64) -> io::Result<Option<u64>> {
        match self.db.get(inode.to_be_bytes())? {
            Some(v) => {
                let bytes: [u8; 8] = v
                    .as_ref()
                    .try_into()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Ok(Some(u64::from_be_bytes(bytes)))
            }
            None => Ok(None),
        }
    }

    pub fn clear_all(&self) -> io::Result<()> {
        self.db.clear().map_err(Error::other)?;
        Ok(())
    }
}
