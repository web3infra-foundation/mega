use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    path::{Path, PathBuf},
    str::FromStr,
};

use git_internal::{
    hash::ObjectHash,
    internal::object::{blob::Blob, commit::Commit, tree::Tree, ObjectTrait},
};
use tokio::sync::mpsc::Receiver;

use crate::util::GPath;

pub trait TreeStore {
    fn insert_tree(&self, path: PathBuf, tree: Tree);
    fn get_bypath(&self, path: &Path) -> Result<Tree>;
    fn db_tree_list(&self) -> Result<HashMap<PathBuf, Tree>>;
}

impl TreeStore for sled::Db {
    fn insert_tree(&self, path: PathBuf, tree: Tree) {
        let config = bincode::config::standard();
        let value = bincode::encode_to_vec(&tree, config).unwrap();
        let key = path.to_str().unwrap();
        self.insert(key, value).unwrap();
    }

    fn get_bypath(&self, path: &Path) -> Result<Tree> {
        let key = path.to_str().unwrap();
        match self.get(key)? {
            Some(encoded_value) => {
                let config = bincode::config::standard();
                let (decoded, _): (Tree, usize) =
                    bincode::decode_from_slice(&encoded_value, config)
                        .map_err(|_| std::io::Error::other("Deserialization error"))?;
                Ok(decoded)
            }
            None => {
                // If the db is empty, return an error not a panic.
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Path '{key}' not found"),
                ))
            }
        }
    }

    fn db_tree_list(&self) -> Result<HashMap<PathBuf, Tree>> {
        self.iter()
            .map(|item| match item {
                // By returning a HashMap, we avoid using a double pointer loop structure in diff.rs.
                Ok((path, encoded_value)) => {
                    // Convert the IVec to a string and then to a PathBuf
                    let path = std::str::from_utf8(&path)
                        .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid UTF8 path"))?;
                    let config = bincode::config::standard();
                    let (decoded_tree, _): (Tree, usize) =
                        bincode::decode_from_slice(&encoded_value, config)
                            .map_err(|_| Error::other("Deserialization error"))?;
                    Ok((PathBuf::from(path), decoded_tree))
                }
                Err(e) => Err(Error::new(ErrorKind::NotFound, e)),
            })
            .collect::<Result<HashMap<PathBuf, Tree>>>()
    }
}

#[allow(unused)]
pub trait CommitStore {
    fn store_commit(&self, commit: Commit) -> Result<()>;
    fn get_commit(&self) -> Result<Commit>;
}
impl CommitStore for sled::Db {
    fn store_commit(&self, commit: Commit) -> Result<()> {
        let config = bincode::config::standard();
        let encoded_commit = bincode::encode_to_vec(&commit, config).unwrap();
        let re = self.insert("COMMIT", encoded_commit)?;
        if re.is_some() {
            Ok(())
        } else {
            Err(std::io::Error::other("Failed to store commit"))
        }
    }

    fn get_commit(&self) -> Result<Commit> {
        let encoded_value = self.get("COMMIT")?;
        let config = bincode::config::standard();
        let (decoded, _): (Commit, usize) =
            bincode::decode_from_slice(&encoded_value.unwrap(), config)
                .map_err(|_| std::io::Error::other("Deserialization error"))?;
        Ok(decoded)
    }
}
pub async fn store_trees(storepath: &str, mut tree_channel: Receiver<(GPath, Tree)>) -> Result<()> {
    let db = sled::open(storepath)?;
    while let Some((path, tree)) = tree_channel.recv().await {
        // println!("new tree:{}", tree.id);
        db.insert_tree(path.into(), tree);
    }

    // println!("finish store....");

    Ok(())
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
pub trait BlobFsStore {
    // This version only defines the add and get APIs for
    // blob ojects processing.

    fn add_blob_to_hash(&self, hash: &str, blob: &[u8]) -> Result<()>;
    fn get_blob_by_hash(&self, hash: &str) -> Result<Vec<u8>>;
    fn list_blobs(&self, index_db: &sled::Db) -> Result<Vec<Blob>>;
}
impl BlobFsStore for PathBuf {
    /// These functions is used to implement a storage
    /// method similar to the Git Blob structure in the
    /// PathBuf structure.
    ///
    /// The PathBuf corresponding to this trait should
    /// be the directory where the "index.db" folder of
    /// the db is located.
    ///
    /// At the same time, for a clear structure, this
    /// version will migrate the db to the "modifystore"
    /// folder.
    ///
    /// Added WhiteOut file support to avoid some unexpected errors.
    /// Add a Blob Object into the folder by a hash.
    ///
    /// **Multi-hash support**: Accepts both SHA-1 (40 chars) and SHA-256 (64 chars) hashes.
    fn add_blob_to_hash(&self, hash: &str, blob: &[u8]) -> Result<()> {
        // **Multi-hash support**: Accept both SHA-1 (40 chars) and SHA-256 (64 chars)
        match hash.len() {
            40 | 64 => (),
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Hash must be 40 (SHA-1) or 64 (SHA-256) characters long",
                ))
            }
        };
        let object_path = self.join("objects");
        let hash_path = object_path.join(&hash[0..2]);
        std::fs::create_dir_all(&hash_path)?;

        let blob_path = hash_path.join(&hash[2..]);
        let compressed_blob = format_data(blob);
        std::fs::write(blob_path, compressed_blob)
    }

    /// Get the content from a Blob Object by a hash.
    ///
    /// **Multi-hash support**: Accepts both SHA-1 (40 chars) and SHA-256 (64 chars) hashes.
    fn get_blob_by_hash(&self, hash: &str) -> Result<Vec<u8>> {
        // **Multi-hash support**: Accept both SHA-1 (40 chars) and SHA-256 (64 chars)
        match hash.len() {
            40 | 64 => {
                let object_path = self.join("objects");
                let hash_path = object_path.join(&hash[0..2]);
                let blob_path = hash_path.join(&hash[2..]);
                let blob = std::fs::read(&blob_path)?;
                Ok(unformat_data(&blob))
            }
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                "Hash must be 40 (SHA-1) or 64 (SHA-256) characters long",
            )),
        }
    }

    /// **Multi-hash support**: Handles both SHA-1 (40 chars) and SHA-256 (64 chars) hashes.
    fn list_blobs(&self, index_db: &sled::Db) -> Result<Vec<Blob>> {
        let hashmap = index_db.db_list()?;
        let object_path = self.join("objects");
        hashmap
            .values()
            .map(|hash| {
                // **Multi-hash support**: First 2 chars for directory, works for both hash types
                let hash_flag = hash.get(0..2).ok_or(Error::new(
                    ErrorKind::InvalidInput,
                    "Hash must be at least 2 characters long",
                ))?;
                let hash_path = object_path.join(hash_flag);
                // ObjectHash::from_str automatically handles both 40 and 64 char hashes
                let sha_hash = ObjectHash::from_str(hash)
                    .map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;

                let data_path = hash_path.join(&hash[2..]);
                let data = std::fs::read(&data_path)?;
                let blob = Blob::from_bytes(&data, sha_hash);
                blob.map_err(|e| Error::new(ErrorKind::InvalidData, e))
            })
            .collect::<Result<Vec<Blob>>>()
    }
}

pub trait ModifiedStore {
    fn add(&self, path: PathBuf) -> Result<()>;
    fn add_content(&self, path: PathBuf, content: &[u8]) -> Result<()>; // if the state of a file is deleted, content is None.
    fn path_list(&self) -> Result<Vec<PathBuf>>;
    fn db_list(&self) -> Result<HashMap<PathBuf, String>>;
    fn get_content(&self, path: PathBuf) -> Result<Vec<u8>>;
    fn delete(&self, path: PathBuf) -> Result<bool>; // true for success, false for no this path.
}
impl ModifiedStore for sled::Db {
    fn add(&self, path: PathBuf) -> Result<()> {
        let key = path.to_str().unwrap();
        self.insert(key, b"")?;
        Ok(())
    }

    fn add_content(&self, path: PathBuf, content: &[u8]) -> Result<()> {
        let key = path.to_str().unwrap();
        self.insert(key, content)?;
        Ok(())
    }

    fn path_list(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for item in self.iter() {
            let (key, _) = item?;
            let key_str =
                std::str::from_utf8(&key).map_err(|_| std::io::Error::other("Invalid UTF8"))?;
            paths.push(PathBuf::from(key_str));
        }
        Ok(paths)
    }

    fn db_list(&self) -> Result<HashMap<PathBuf, String>> {
        self.iter()
            .map(|item| match item {
                // By returning a HashMap, we avoid using a double pointer loop structure in diff.rs.
                Ok((path, hash)) => {
                    // Convert the IVec to a string and then to a PathBuf
                    let path = std::str::from_utf8(&path).unwrap_or("Invalid UTF8 path");
                    let hash = std::str::from_utf8(&hash)
                        .unwrap_or("Invalid UTF8 path")
                        .to_string();
                    Ok((PathBuf::from(path), hash))
                }
                Err(e) => Err(Error::other(e)),
            })
            .collect::<Result<HashMap<PathBuf, String>>>()
    }

    fn get_content(&self, path: PathBuf) -> Result<Vec<u8>> {
        let key = path.to_str().unwrap();
        if let Some(content) = self.get(key)? {
            Ok(content.to_vec())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Path not found",
            ))
        }
    }

    fn delete(&self, path: PathBuf) -> Result<bool> {
        let key = path.to_str().unwrap();
        let removed = self.remove(key)?;
        Ok(removed.is_some())
    }
}

pub struct TempStoreArea {
    pub index_db: sled::Db,
    pub rm_db: sled::Db,
}

impl TempStoreArea {
    pub fn new(modified_path: &Path) -> Result<Self> {
        let index_db = sled::open(modified_path.join("index.db"))?;
        let rm_db = sled::open(modified_path.join("removedfile.db"))?;
        Ok(Self { index_db, rm_db })
    }
}

// #[allow(unused)]
// pub struct TreesStore<T: kv::KvStore<PathBuf, Tree>> {
//     db: T,
// }

// #[allow(unused)]
// impl<T: kv::KvStore<PathBuf, Tree>> TreesStore<T> {
//     pub fn new(db: T) -> Self {
//         TreesStore { db }
//     }
//     fn insert_tree(&self, path: PathBuf, tree: Tree) -> Result<()> {
//         self.db._set(path, tree)?;
//         Ok(())
//     }

//     fn get_bypath(&self, path: PathBuf) -> Result<Tree> {
//         match self.db._get(&path).map_err(std::io::Error::other)? {
//             Some(encoded_value) => Ok(encoded_value),
//             None => Err(std::io::Error::new(
//                 std::io::ErrorKind::NotFound,
//                 format!("Path '{}' not found", path.to_str().unwrap()),
//             )),
//         }
//     }
// }

// mod kv {

//     use bincode::{config, error::EncodeError, Decode, Encode};
//     use fjall::{Config, PartitionCreateOptions, PersistMode};
//     use serde::{de::DeserializeOwned, Serialize};
//     use std::{marker::PhantomData, path::Path};
//     use thiserror::Error;

//     /// A generic key-value store trait with automatic serialization/deserialization.
//     ///
//     /// This trait provides a common interface for key-value storage implementations,
//     /// handling serialization and deserialization of keys and values transparently.
//     /// It is designed to work with types that implement Serde's serialization traits.
//     ///
//     /// # Type Parameters
//     /// - `K`: Key type implementing Serialize and DeserializeOwned
//     /// - `V`: Value type implementing Serialize and DeserializeOwned
//     ///
//     /// # Usage
//     /// Implement this trait for different storage backends while maintaining
//     /// a consistent interface for key-value operations.
//     pub trait KvStore<K, V>
//     where
//         K: Encode+ Decode<()>,
//         V: Encode+ Decode<()>,
//     {
//         /// Inserts or updates a key-value pair (automatic serialization)
//         ///
//         /// # Arguments
//         /// * `key` - Key to insert/update
//         /// * `value` - Value to associate with the key
//         ///
//         /// # Errors
//         /// Returns `KvError` for serialization failures or storage errors
//         fn _set(&self, key: K, value: V) -> Result<(), KvError>;

//         /// Retrieves the value associated with the key (automatic deserialization)
//         ///
//         /// # Arguments
//         /// * `key` - Key to look up
//         ///
//         /// # Returns
//         /// `Ok(Some(V))` if key exists, `Ok(None)` if not found
//         ///
//         /// # Errors
//         /// Returns `KvError` for deserialization failures or storage errors
//         fn _get(&self, key: &K) -> Result<Option<V>, KvError>;

//         /// Removes a key-value pair from the store
//         ///
//         /// # Arguments
//         /// * `key` - Key to remove
//         ///
//         /// # Errors
//         /// Returns `KvError` if removal fails
//         fn _remove(&self, key: &K) -> Result<(), KvError>;

//         /// Checks existence of a key in the store
//         ///
//         /// # Arguments
//         /// * `key` - Key to check
//         ///
//         /// # Returns
//         /// `true` if key exists, `false` otherwise
//         ///
//         /// # Errors
//         /// Returns `KvError` for storage operation failures
//         fn _contains_key(&self, key: &K) -> Result<bool, KvError>;

//         /// Clears all key-value pairs from the store
//         ///
//         /// # Errors
//         /// Returns `KvError` if clear operation fails
//         fn _clear(&self) -> Result<(), KvError>;
//     }

//     #[derive(Error, Debug)]
//     pub enum KvError {
//         #[error("Deserialization error: {0}")]
//         Deserialization(String),

//         #[error("I/O error: {0}")]
//         IoError(#[from] std::io::Error),

//         #[error("Serialization error: {0}")]
//         Serialization(#[from] EncodeError),

//         #[error("Fjall error: {0}")]
//         FjallError(String),

//         #[error("Other error: {0}")]
//         Other(#[from] Box<dyn std::error::Error + Send + Sync>),
//     }

//     impl From<fjall::Error> for KvError {
//         fn from(e: fjall::Error) -> Self {
//             KvError::FjallError(e.to_string())
//         }
//     }

//     impl From<KvError> for std::io::Error {
//         fn from(e: KvError) -> Self {
//             match e {
//                 KvError::IoError(e) => e,
//                 _ => std::io::Error::other(e),
//             }
//         }
//     }

//     impl<K, V> KvStore<K, V> for sled::Db
//     where
//         K: Encode + Decode<()>,
//         V: Encode + Decode<()>,
//     {
//         fn _set(&self, key: K, value: V) -> Result<(), KvError> {
//             let config = config::standard();
//             let serialized_key = bincode::encode_to_vec(&key, config).map_err(KvError::Serialization)?;
//             let serialized_value = bincode::encode_to_vec(&value, config).map_err(KvError::Serialization)?;

//             self.insert(serialized_key, serialized_value)
//                 .map_err(|e| KvError::IoError(e.into()))?;

//             Ok(())
//         }

//         fn _get(&self, key: &K) -> Result<Option<V>, KvError> {
//             let config = config::standard();
//             let serialized_key = bincode::encode_to_vec(key,config).map_err(KvError::Serialization)?;

//             match self
//                 .get(&serialized_key)
//                 .map_err(|e| KvError::IoError(e.into()))?
//             {
//                 Some(value) => {
//                     let config = config::standard();
//                     let (deserialized, _): (V, usize) = bincode::decode_from_slice(&value, config)
//                         .map_err(|e| KvError::Deserialization(e.to_string()))?;
//                     Ok(Some(deserialized))
//                 }
//                 None => Ok(None),
//             }
//         }

//         fn _remove(&self, key: &K) -> Result<(), KvError> {
//             let config = config::standard();
//             let serialized_key = bincode::encode_to_vec(key, config).map_err(KvError::Serialization)?;

//             self.remove(serialized_key)
//                 .map_err(|e| KvError::IoError(e.into()))?;

//             Ok(())
//         }

//         fn _contains_key(&self, key: &K) -> Result<bool, KvError> {
//             let config = config::standard();
//             let serialized_key = bincode::encode_to_vec(key, config).map_err(KvError::Serialization)?;

//             self.contains_key(serialized_key)
//                 .map_err(|e| KvError::IoError(e.into()))
//         }

//         fn _clear(&self) -> Result<(), KvError> {
//             self.clear().map_err(|e| KvError::IoError(e.into()))?;
//             Ok(())
//         }
//     }

//     pub struct FjallKvStore<K, V> {
//         keyspace: fjall::Keyspace,
//         partition_name: String,
//         _key_type: PhantomData<K>,
//         _value_type: PhantomData<V>,
//     }

//     #[allow(unused)]
//     impl<K, V> FjallKvStore<K, V>
//     where
//         K: Serialize + DeserializeOwned  ,
//         V: Serialize + DeserializeOwned ,
//     {
//         pub fn new<P: AsRef<Path>>(path: P, partition_name: &str) -> Result<Self, KvError> {
//             let keyspace = Config::new(path).open()?;
//             keyspace.persist(PersistMode::Buffer)?;

//             Ok(Self {
//                 keyspace,
//                 partition_name: partition_name.to_string(),
//                 _key_type: PhantomData,
//                 _value_type: PhantomData,
//             })
//         }

//         // pub fn new_transactional<P: AsRef<Path>>(path: P, partition_name: &str) -> Result<Self, KvError> {
//         //     let keyspace = Config::new(path).open_transactional()?;
//         //     Ok(Self {
//         //         keyspace,
//         //         partition_name: partition_name.to_string(),
//         //         _key_type: PhantomData,
//         //         _value_type: PhantomData,
//         //     })
//         // }

//         fn open_partition(&self) -> Result<fjall::PartitionHandle, KvError> {
//             self.keyspace
//                 .open_partition(&self.partition_name, PartitionCreateOptions::default())
//                 .map_err(Into::into)
//         }
//     }

//     impl<K, V> KvStore<K, V> for FjallKvStore<K, V>
//     where
//         K: Encode + Decode<()> + DeserializeOwned,
//         V: Encode + Decode<()> + DeserializeOwned,
//     {
//         fn _set(&self, key: K, value: V) -> Result<(), KvError> {
//             let config = bincode::config::standard();
//             let serialized_key = bincode::encode_to_vec(&key,config)?;
//             let serialized_value = bincode::encode_to_vec(&value,config)?;

//             let partition = self.open_partition()?;
//             partition.insert(&serialized_key, &serialized_value)?;
//             Ok(())
//         }

//         fn _get(&self, key: &K) -> Result<Option<V>, KvError> {
//             let config = bincode::config::standard();
//             let serialized_key = bincode::encode_to_vec(key,config)?;
//             let partition = self.open_partition()?;

//             match partition.get(&serialized_key)? {
//                 Some(v) => {
//                     let config = bincode::config::standard();
//                     let (deserialized, _): (V, usize) = bincode::decode_from_slice(&v, config)
//                         .map_err(|e| KvError::Deserialization(e.to_string()))?;
//                     Ok(Some(deserialized))
//                 }
//                 None => Ok(None),
//             }
//         }

//         fn _remove(&self, key: &K) -> Result<(), KvError> {
//             let serialized_key = bincode::serialize(key)?;

//             let partition = self.open_partition()?;
//             partition.remove(&serialized_key)?;
//             Ok(())
//         }

//         fn _contains_key(&self, key: &K) -> Result<bool, KvError> {
//             let config = config::standard();
//             let serialized_key = bincode::encode_to_vec(key,config)?;

//             let partition = self.open_partition()?;
//             Ok(partition.get(&serialized_key)?.is_some())
//         }

//         fn _clear(&self) -> Result<(), KvError> {
//             let partition = self.open_partition()?;

//             // Attention: this may take a lot
//             let _ = partition
//                 .iter()
//                 .map(|res| res.map(|(k, _)| partition.remove(k)));
//             Ok(())
//         }
//     }
// }

#[cfg(test)]
mod test {
    use std::vec;

    use git_internal::{
        hash::ObjectHash,
        internal::object::tree::{Tree, TreeItem, TreeItemMode},
    };

    #[test]
    fn init_test_d() {
        let db_path = "/tmp/init_test_d.db";
        if std::path::Path::new(db_path).exists() {
            std::fs::remove_file(db_path).ok();
        }
        let db = sled::open(db_path).unwrap();
        let t = Tree::from_tree_items(vec![TreeItem::new(
            TreeItemMode::Blob,
            ObjectHash::new(&[4u8, 4u8, 4u8, 64u8, 84u8, 84u8]),
            String::from("test"),
        )])
        .unwrap();

        if let Some(encoded_value) = db.get(t.id.as_ref()).unwrap() {
            // use bincode to deserialize the value .
            let config = bincode::config::standard();
            let decoded: Tree = bincode::decode_from_slice(&encoded_value, config)
                .unwrap()
                .0;
            println!(" {decoded}");
        };
    }

    #[test]
    #[ignore = "manual test requiring specific database file"]
    fn get_tree_test() {
        // Use environment variable or default path for flexibility
        let db_path =
            std::env::var("TREE_DB_PATH").unwrap_or_else(|_| "/tmp/test_tree.db".to_string());
        let db = sled::open(&db_path).unwrap();
        let iter = db.iter();
        for result in iter {
            match result {
                Ok((key, value)) => {
                    // Deserialize the value into the original tree structure using bincode
                    let config = bincode::config::standard();
                    let tree: Tree = bincode::decode_from_slice(&value, config).unwrap().0;
                    let key_str = std::str::from_utf8(&key).unwrap();

                    println!("path:{key_str}");
                    println!("{tree}");
                }
                Err(error) => {
                    println!("Error iterating over trees: {error}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_add_blob_sha1_40_chars() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_path_buf();

        // 40-char SHA-1 hash
        let sha1_hash = "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3";
        let data = b"test content";

        let result = store_path.add_blob_to_hash(sha1_hash, data);
        assert!(result.is_ok(), "SHA-1 (40 chars) should be accepted");
    }

    #[test]
    fn test_add_blob_sha256_64_chars() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_path_buf();

        // 64-char SHA-256 hash
        let sha256_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let data = b"test content";

        let result = store_path.add_blob_to_hash(sha256_hash, data);
        assert!(result.is_ok(), "SHA-256 (64 chars) should be accepted");
    }

    #[test]
    fn test_add_blob_invalid_length() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_path_buf();

        // Invalid length (not 40 or 64)
        let invalid_hash = "abc123";
        let data = b"test content";

        let result = store_path.add_blob_to_hash(invalid_hash, data);
        assert!(result.is_err(), "Invalid hash length should be rejected");
    }

    #[test]
    fn test_get_blob_sha1_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_path_buf();

        let sha1_hash = "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3";
        let data = b"test content for sha1";

        store_path.add_blob_to_hash(sha1_hash, data).unwrap();
        let retrieved = store_path.get_blob_by_hash(sha1_hash).unwrap();

        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_get_blob_sha256_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_path_buf();

        let sha256_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let data = b"test content for sha256";

        store_path.add_blob_to_hash(sha256_hash, data).unwrap();
        let retrieved = store_path.get_blob_by_hash(sha256_hash).unwrap();

        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_get_blob_invalid_length() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().to_path_buf();

        let invalid_hash = "abc123";

        let result = store_path.get_blob_by_hash(invalid_hash);
        assert!(result.is_err(), "Invalid hash length should be rejected");
    }
}
