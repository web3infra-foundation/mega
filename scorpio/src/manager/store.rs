use mercury::internal::object::{commit::Commit, tree::Tree};
use sled::IVec;
use std::collections::HashMap;
use std::fmt::Display;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use tokio::sync::mpsc::Receiver;

use crate::util::GPath;

pub trait TreeStore {
    fn insert_tree(&self, path: PathBuf, tree: Tree) -> Result<()>;
    fn get_bypath(&self, path: PathBuf) -> Result<Tree>;
}

impl TreeStore for sled::Db {
    fn insert_tree(&self, path: PathBuf, tree: Tree) -> Result<()> {
        let value = bincode::serialize(&tree).unwrap();
        let key = path.to_str().unwrap();
        self.insert(key, value).unwrap();
        Ok(())
    }

    fn get_bypath(&self, path: PathBuf) -> Result<Tree> {
        let key = path.to_str().unwrap();
        match self.get(key)? {
            Some(encoded_value) => {
                let decoded: Result<Tree> = bincode::deserialize(&encoded_value)
                    .map_err(|_| std::io::Error::other("Deserialization error"));
                let decoded: Tree = decoded?;
                Ok(decoded)
            }
            None => {
                // If the db is empty, return an error not a panic.
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Path '{}' not found", key),
                ))
            }
        }
    }
}
#[allow(unused)]
pub trait CommitStore {
    fn store_commit(&self, commit: Commit) -> Result<()>;
    fn get_commit(&self) -> Result<Commit>;
}
impl CommitStore for sled::Db {
    fn store_commit(&self, commit: Commit) -> Result<()> {
        let re = self.insert("COMMIT", bincode::serialize(&commit).unwrap())?;
        if re.is_some() {
            Ok(())
        } else {
            Err(std::io::Error::other("Failed to store commit"))
        }
    }

    fn get_commit(&self) -> Result<Commit> {
        let encoded_value = self.get("COMMIT")?;
        let decoded: Result<Commit> = bincode::deserialize(&encoded_value.unwrap())
            .map_err(|_| std::io::Error::other("Deserialization error"));
        decoded
    }
}
pub async fn store_trees(storepath: &str, mut tree_channel: Receiver<(GPath, Tree)>) {
    let db = sled::open(storepath).unwrap();
    while let Some((path, tree)) = tree_channel.recv().await {
        println!("new tree:{}", tree.id);
        let re = db.insert_tree(path.into(), tree);
        if re.is_err() {
            print!("{}", re.err().unwrap());
        }
    }

    println!("finish store....");
}

// To avoid defining too many traits, enumeration
// is now used for unified optimization.
pub enum StorageSpace {
    SledDb(sled::Db),
    SledBat(sled::Batch),
    BlobFs(PathBuf),
}

/// Display trait for StorageSpace type
impl Display for StorageSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StorageSpace::SledDb(_) => write!(f, "sled::Db"),
            StorageSpace::SledBat(_) => write!(f, "sled::Batch"),
            StorageSpace::BlobFs(_) => write!(f, "std::path::PathBuf"),
        }
    }
}

// Some possible common methods
impl StorageSpace {
    /// Try converting to Db type
    pub fn try_to_db(&self) -> Result<&sled::Db> {
        match self {
            StorageSpace::SledDb(db) => Ok(db),
            _ => Err(MKVSError::UnsupportedError.into()),
        }
    }

    /// Try converting to Batch type
    pub fn try_to_bat(&mut self) -> Result<&mut sled::Batch> {
        match self {
            StorageSpace::SledBat(batch) => Ok(batch),
            _ => Err(MKVSError::UnsupportedError.into()),
        }
    }

    /// Try converting to PathBuf type
    pub fn try_to_path(&self) -> Result<&PathBuf> {
        match self {
            StorageSpace::BlobFs(path) => Ok(path),
            _ => Err(MKVSError::UnsupportedError.into()),
        }
    }
}

/// This enumeration contains the error types and return
/// values ​​used in the ModifiedKVStore feature.
enum MKVSError {
    UnsupportedError,
    InvalidDataError(String),
    InvalidInputError(String),
    NotFoundError,
}

impl From<MKVSError> for std::io::Error {
    fn from(err: MKVSError) -> Self {
        match err {
            MKVSError::UnsupportedError => {
                Error::new(ErrorKind::Unsupported, "Unsupported Operation")
            }
            MKVSError::InvalidDataError(msg) => Error::new(ErrorKind::InvalidData, msg),
            MKVSError::InvalidInputError(msg) => Error::new(ErrorKind::InvalidInput, msg),
            MKVSError::NotFoundError => Error::new(ErrorKind::NotFound, "Path not found"),
        }
    }
}

// This function is used to format the data to Git Blob format.
fn format_data(data: &[u8]) -> Box<[u8]> {
    let header: Vec<u8> = format!("blob {}", data.len()).into_bytes();
    let mut res: Vec<u8> = Vec::with_capacity(header.len() + 1 + data.len());
    res.extend(header);
    res.push(b'\x00');
    res.extend_from_slice(data);
    Box::from(res.as_slice())
}

// This function is used to unformat the data from Git Blob format.
fn unformat_data(data: &[u8]) -> Vec<u8> {
    // The first part is the header, and the second part is the actual data.
    match data.is_empty() {
        true => Vec::new(),
        false => {
            let index: usize = data
                .iter()
                .position(|&b| b == b'\x00')
                .expect("No null byte found in data");
            data[(index + 1)..].to_vec()
        }
    }
}

/// This trait implements the CRUD API for sled::Db and
/// sled::Batch, as well as a storage mechanism similar
/// to Git Blobs objects.
pub trait ModifiedKVStore {
    const WHITEOUT_FLAG: &'static str;

    // This version defines a set of add, delete, and modify
    // APIs for databases and batch processing.
    //
    // Although we could combine the two sets of functionality
    // into one, we chose to implement them separately for safety
    // reasons and Rust's design philosophy of independence and
    // clarity.

    // Although this set of functions returns Result, it is
    // recommended to use .unwrap() instead of ? to check the
    // result.
    fn bat_add_kv(&mut self, path: &PathBuf, hash: &str);
    fn bat_add_whiteout(&mut self, path: &PathBuf);
    fn bat_del_kv(&mut self, path: &PathBuf);

    fn db_add_k(&self, path: &PathBuf) -> Result<()>;
    fn db_add_kv(&self, path: &PathBuf, hash: &str) -> Result<()>; // if the state of a file is deleted, content is None.
    fn db_add_whiteout(&self, path: &PathBuf) -> Result<()>;
    fn db_del_kv(&self, path: &PathBuf) -> Result<bool>; // true for success, false for no this path.
    fn db_get_kv(&self, path: &PathBuf) -> Result<String>;

    fn list_whiteout_file(&self) -> Result<Vec<PathBuf>>;
    fn list_keys(&self) -> Result<Vec<PathBuf>>;
    fn list_values(&self) -> Result<HashMap<String, usize>>;
    fn list_db(&self) -> Result<HashMap<PathBuf, String>>;

    fn add_blob_to_hash(&self, hash: &str, blob: &[u8]) -> Result<()>;
    fn get_blob_by_hash(&self, hash: &str) -> Result<Vec<u8>>; // return the blob content as a boxed slice
    fn del_blob_by_hash(&self, hash: &str) -> Result<()>;
}
impl ModifiedKVStore for StorageSpace {
    const WHITEOUT_FLAG: &'static str = r##"WHITEOUT"##;

    // This group is used for BATCH processing.
    // Contains add and delete operations.

    /// Add key-value ​​to a batch.
    fn bat_add_kv(&mut self, path: &PathBuf, hash: &str) {
        let key = path.to_str().unwrap();
        self.try_to_bat().unwrap().insert(key, hash);
    }

    /// Add WhiteOut file to a batch.
    fn bat_add_whiteout(&mut self, path: &PathBuf) {
        self.bat_add_kv(path, StorageSpace::WHITEOUT_FLAG)
    }

    /// Remove a key-value from a batch.
    fn bat_del_kv(&mut self, path: &PathBuf) {
        let key = path.to_str().unwrap();
        self.try_to_bat().unwrap().remove(key)
    }

    // ##################################################
    // This group is used for DATABASE processing.
    // Contains add, delete, get and list operations.

    /// Add an empty key ​​to the db.
    fn db_add_k(&self, path: &PathBuf) -> Result<()> {
        let key = path.to_str().unwrap();
        self.try_to_db()?.insert(key, b"")?;
        Ok(())
    }

    /// Add key-value ​​to the db.
    fn db_add_kv(&self, path: &PathBuf, hash: &str) -> Result<()> {
        let key = path.to_str().unwrap();
        self.try_to_db()?.insert(key, hash)?;
        Ok(())
    }

    /// Add WhiteOut file to the db.
    fn db_add_whiteout(&self, path: &PathBuf) -> Result<()> {
        self.db_add_kv(path, StorageSpace::WHITEOUT_FLAG)
    }

    /// Remove a key-value from the db.
    fn db_del_kv(&self, path: &PathBuf) -> Result<bool> {
        let key = path.to_str().unwrap();
        let removed = self.try_to_db()?.remove(key)?;
        Ok(removed.is_some())
    }

    /// Extract value from database using a key.
    fn db_get_kv(&self, path: &PathBuf) -> Result<String> {
        let key = path.to_str().unwrap();
        match self.try_to_db()?.get(key)? {
            Some(hash) => {
                let hash_string = String::from_utf8(hash.to_vec())
                    .map_err(|_| MKVSError::InvalidDataError("Invalid UTF8 hash".to_string()))?;
                Ok(hash_string)
            }
            None => Err(MKVSError::NotFoundError.into()),
        }
    }

    /// List all of the WhiteOut files in the db
    fn list_whiteout_file(&self) -> Result<Vec<PathBuf>> {
        self.try_to_db()?
            .iter()
            .filter(|item| match item {
                Ok((_, value)) => sled::IVec::from(StorageSpace::WHITEOUT_FLAG).eq(&value),
                Err(_) => false,
            })
            .map(|item| match item {
                Ok((path, _)) => {
                    // Convert the IVec to a string and then to a PathBuf
                    let path = std::str::from_utf8(&path).unwrap_or("Invalid UTF8 path");
                    Ok(PathBuf::from(path))
                }
                Err(e) => Err(MKVSError::InvalidDataError(e.to_string()).into()),
            })
            .collect::<Result<Vec<PathBuf>>>()
    }

    // After careful consideration, I decided that in this version,
    // the return value of the list_keys() function will no longer
    // include the WhiteOut file path. Correspondingly, the list_db()
    // and list_value() functions will also be adjusted.

    /// List the keys ​​of all key-value pairs stored in the database.
    fn list_keys(&self) -> Result<Vec<PathBuf>> {
        self.try_to_db()?
            .iter()
            .filter_map(|item| match item {
                Ok((path, hash)) => match hash.eq(&IVec::from(StorageSpace::WHITEOUT_FLAG)) {
                    true => None,
                    false => {
                        // Convert the IVec to a string and then to a PathBuf
                        let path = std::str::from_utf8(&path).unwrap_or("Invalid UTF8 path");
                        Some(Ok(PathBuf::from(path)))
                    }
                },
                Err(e) => Some(Err(MKVSError::InvalidDataError(e.to_string()).into())),
            })
            .collect::<Result<Vec<PathBuf>>>()
    }

    /// List the values ​​of all key-value pairs stored in the database.
    /// This function could be used to count the repeated values.
    fn list_values(&self) -> Result<HashMap<String, usize>> {
        let mut res: HashMap<String, usize> = HashMap::new();
        for item in self.try_to_db()?.iter() {
            match item {
                Ok((_, hash)) => match hash.eq(&IVec::from(StorageSpace::WHITEOUT_FLAG)) {
                    true => continue,
                    false => {
                        let hash = std::str::from_utf8(&hash)
                            .unwrap_or("Invalid UTF8 path")
                            .to_string();
                        let count = res.entry(hash).or_insert(0);
                        *count += 1;
                    }
                },
                Err(e) => return Err(Error::from(e)),
            }
        }
        Ok(res)
    }

    /// Export all key-value pairs stored in the database into a HashMap.
    fn list_db(&self) -> Result<HashMap<PathBuf, String>> {
        self.try_to_db()?
            .iter()
            .filter_map(|item| match item {
                // By returning a HashMap, we avoid using a double pointer loop structure in diff.rs.
                Ok((path, hash)) => match hash.eq(&IVec::from(StorageSpace::WHITEOUT_FLAG)) {
                    true => None,
                    false => {
                        // Convert the IVec to a string and then to a PathBuf
                        let path = std::str::from_utf8(&path).unwrap_or("Invalid UTF8 path");
                        let path = PathBuf::from(path);
                        let hash = std::str::from_utf8(&hash)
                            .unwrap_or("Invalid UTF8 path")
                            .to_string();
                        Some(Ok((path, hash)))
                    }
                },
                Err(e) => Some(Err(MKVSError::InvalidDataError(e.to_string()).into())),
            })
            .collect::<Result<HashMap<PathBuf, String>>>()
    }

    // ##################################################
    // This group is used for BLOBFS processing.
    // Contains add and delete operations.

    /// These functions is used to implement a storage
    /// method similar to the Git Blob structure in the
    /// PathBuf structure.
    ///
    /// The PathBuf corresponding to this trait should
    /// be the directory where the "index.db" folder of
    /// the db is located.
    ///
    /// At the same time, for a clear structure, this
    /// version will migrate the db to the "ModifyStore"
    /// folder.
    ///
    /// Added WhiteOut file support to avoid some unexpected errors.
    /// Add a Blob Object into the folder by a hash.
    fn add_blob_to_hash(&self, hash: &str, blob: &[u8]) -> Result<()> {
        match (hash, hash.len()) {
            (StorageSpace::WHITEOUT_FLAG, _) => println!("Add new content to a WhiteOut file."),
            (_, 40) => (),
            _ => {
                return Err(MKVSError::InvalidInputError(
                    "Hash must be 40 characters long".to_string(),
                )
                .into())
            }
        };
        let object_path = self.try_to_path()?.join("objects");
        let hash_path = object_path.join(&hash[0..2]);
        std::fs::create_dir_all(&hash_path)?;

        let blob_path = hash_path.join(&hash[2..]);
        let compressed_blob = format_data(blob);
        std::fs::write(blob_path, compressed_blob)
    }

    /// Get the content from a Blob Object by a hash.
    fn get_blob_by_hash(&self, hash: &str) -> Result<Vec<u8>> {
        match (hash, hash.len()) {
            (_, 40) => {
                let object_path = self.try_to_path()?.join("objects");
                let hash_path = object_path.join(&hash[0..2]);
                let blob_path = hash_path.join(&hash[2..]);
                let blob = std::fs::read(&blob_path)?;
                Ok(unformat_data(&blob))
            }
            (StorageSpace::WHITEOUT_FLAG, _) => {
                let e_message = "Trying to export the content of a WhiteOut file, stop.";
                println!("{e_message}");
                Err(MKVSError::InvalidDataError(e_message.to_string()).into())
            }
            _ => Err(
                MKVSError::InvalidInputError("Hash must be 40 characters long".to_string()).into(),
            ),
        }
    }

    /// Delete a Blob Object and the stored folder.
    fn del_blob_by_hash(&self, hash: &str) -> Result<()> {
        match (hash, hash.len()) {
            (StorageSpace::WHITEOUT_FLAG, _) => Ok(()),
            (_, 40) => {
                let object_path = self.try_to_path()?.join("objects");
                let hash_path = object_path.join(&hash[0..2]);
                std::fs::remove_dir_all(&hash_path)
            }
            _ => Err(
                MKVSError::InvalidInputError("Hash must be 40 characters long".to_string()).into(),
            ),
        }
    }
}

#[cfg(test)]
mod test {
    use mercury::{
        hash::SHA1,
        internal::object::tree::{Tree, TreeItem, TreeItemMode},
    };
    use std::vec;

    #[test]
    fn init_test_d() {
        let db = sled::open("path.db").unwrap();
        let t = Tree::from_tree_items(vec![TreeItem::new(
            TreeItemMode::Blob,
            SHA1::new(&[4u8, 4u8, 4u8, 64u8, 84u8, 84u8]),
            String::from("test"),
        )])
        .unwrap();

        if let Some(encoded_value) = db.get(t.id.as_ref()).unwrap() {
            // use bincode to deserialize the value .
            let decoded: Tree = bincode::deserialize(&encoded_value).unwrap();
            println!(" {}", decoded);
        };
    }

    #[test]
    fn get_tree_test() {
        let db = sled::open(
            "/home/luxian/megadir/store/1b70e8bf4d39d6f5e9dd1637aaa2c221e2d00a27/tree.db",
        )
        .unwrap();
        let iter = db.iter();
        for result in iter {
            match result {
                Ok((key, value)) => {
                    // Deserialize the value into the original tree structure using bincode
                    let decoded: Result<Tree, _> = bincode::deserialize(&value);
                    let key_str = std::str::from_utf8(&key).unwrap();

                    println!("path:{}", key_str);

                    if let Ok(tree) = decoded {
                        println!("{}", tree);
                    } else {
                        println!(
                            "Deserialization error for key: {}",
                            String::from_utf8_lossy(&key)
                        );
                    }
                }
                Err(error) => {
                    println!("Error iterating over trees: {}", error);
                }
            }
        }
    }
}
