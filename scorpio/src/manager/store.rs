use mercury::internal::object::{commit::Commit, tree::Tree};
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::Receiver;

use crate::util::GPath;

pub trait TreeStore {
    fn insert_tree(&self, path: PathBuf, tree: Tree);
    fn get_bypath(&self, path: &Path) -> Result<Tree>;
}

impl TreeStore for sled::Db {
    fn insert_tree(&self, path: PathBuf, tree: Tree) {
        let value = bincode::serialize(&tree).unwrap();
        let key = path.to_str().unwrap();
        self.insert(key, value).unwrap();
    }

    fn get_bypath(&self, path: &Path) -> Result<Tree> {
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
pub async fn store_trees(storepath: &str, mut tree_channel: Receiver<(GPath, Tree)>) -> Result<()> {
    let db = sled::open(storepath)?;
    while let Some((path, tree)) = tree_channel.recv().await {
        println!("new tree:{}", tree.id);
        db.insert_tree(path.into(), tree);
    }

    println!("finish store....");

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
    fn add_blob_to_hash(&self, hash: &str, blob: &[u8]) -> Result<()> {
        match hash.len() {
            40 => (),
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Hash must be 40 characters long",
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
    fn get_blob_by_hash(&self, hash: &str) -> Result<Vec<u8>> {
        match hash.len() {
            40 => {
                let object_path = self.join("objects");
                let hash_path = object_path.join(&hash[0..2]);
                let blob_path = hash_path.join(&hash[2..]);
                let blob = std::fs::read(&blob_path)?;
                Ok(unformat_data(&blob))
            }
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                "Hash must be 40 characters long",
            )),
        }
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
                Err(e) => Err(Error::new(ErrorKind::Other, e)),
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
