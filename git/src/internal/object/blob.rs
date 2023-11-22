//! In Git, a blob (binary large object) is a type of Git object that stores the contents of a file.
//! A blob object contains the binary data of a file, but does not contain any metadata such as
//! the file name or permissions. The structure of a Git blob object is as follows:
//!
//! ```bash
//! blob <content-length>\0<content>
//! ```
//!
//! - `blob` is the object type, indicating that this is a blob object.
//! - `<content-length>` is the length of the content in bytes, encoded as a string of decimal digits.
//! - `\0` is a null byte, which separates the header from the content.
//! - `<content>` is the binary data of the file, represented as a sequence of bytes.
//!
//! We can create a Git blob object for this file by running the following command:
//!
//! ```bash
//! $ echo "Hello, world!" | git hash-object -w --stdin
//! ```
//!
//! This will output a SHA-1 hash, which is the ID of the newly created blob object.
//! The contents of the blob object would look something like this:
//!
//! ```bash
//! blob 13\0Hello, world!
//! ```
//! Git uses blobs to store the contents of files in a repository. Each version of a file is
//! represented by a separate blob object, which can be linked together using Git's commit and tree
//! objects to form a version history of the repository.
//!

use std::fmt::Display;
use std::str;

use crate::errors::GitError;
use crate::hash::Hash;
use crate::internal::object::tree::{TreeItem, TreeItemMode};
use crate::internal::object::ObjectT;
use crate::internal::ObjectType;

/// **The Blob Object**
///
/// # Attention
/// 1. The blob content is stored in the Meta object, so the Blob object only stores the Meta object.
/// 2. When the object saving to the disk, the Git use zip compression algorithm to compress.
#[allow(unused)]
#[derive(Eq, Debug, Clone)]
pub struct Blob {
    pub id: Hash,
    pub data: Vec<u8>,
}

impl PartialEq for Blob {
    /// The Blob object is equal to another Blob object if their IDs are equal.
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Display for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Hash: {}", self.id).unwrap();
        writeln!(f, "Type: Blob").unwrap();
        writeln!(f, "Size: {}", self.data.len())
        // writeln!(f, "{}", String::from_utf8_lossy(&self.data).to_string())
    }
}

impl Blob {
    #[allow(unused)]
    pub fn to_data(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// Generate a tree item string for the Blob object.
    #[allow(unused)]
    pub fn generate_tree_item(&self, filename: &str) -> Result<TreeItem, GitError> {
        Ok(TreeItem {
            mode: TreeItemMode::Blob,
            id: self.id,
            name: filename.to_string(),
        })
    }
}

impl ObjectT for Blob {
    fn get_hash(&self) -> Hash {
        self.id
    }

    fn get_raw(&self) -> Vec<u8> {
        self.data.clone()
    }

    fn get_type(&self) -> crate::internal::ObjectType {
        ObjectType::Blob
    }

    fn set_hash(&mut self, h: Hash) {
        self.id = h;
    }

    /// Create a new Blob object from a data.
    #[allow(unused)]
    fn new_from_data(content: Vec<u8>) -> Self {
        Self {
            id: Hash([0u8; 20]),
            data: content,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::env;
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use crate::internal::object::blob::Blob;
    use crate::internal::object::meta::Meta;
    use crate::internal::object::ObjectT;
    use crate::internal::zlib::stream::inflate::ReadBoxed;
    use crate::internal::ObjectType;
    use crate::utils;

    #[test]
    fn test_new_from_meta() {
        let t_test = Cursor::new(utils::compress_zlib("Hello, World!".as_bytes()).unwrap());
        let mut deco = ReadBoxed::new(t_test, ObjectType::Blob, 13);

        let _blob = Blob::new_from_read(&mut deco, 13);
        assert_eq!(
            _blob.id.to_plain_str(),
            "b45ef6fec89518d314f546fd6c3025367b721684"
        );
        let rrr: Arc<Mutex<dyn Any>> = Arc::new(Mutex::new(_blob));
        let mut binding = rrr.lock().unwrap();
        let bb = binding.downcast_mut::<Blob>().unwrap();
        print!("{}", bb);
    }

    #[test]
    fn test_real_blob() {
        let content = String::from(
            r#"[package]
name = "mega"
version = "0.1.0"
edition = "2021"

[workspace]
members = [".","gateway", "git", "megacore", "storage"]

[dependencies]
gateway = { path = "gateway" }
megacore = { path = "megacore" }

config = "0.13.3"
serde = { version = "1.0.163", features = ["derive"] }
serde_json = "1.0.96"
clap = { version = "4.3.0", features = ["derive"] }
anyhow = "1.0.69"
lazy_static = "1.4.0"
shadow-rs = "0.23.0"
tokio = {version = "1.28.1", features = ["full"]}
dotenvy = "0.15.7"
tracing-subscriber = "0.3.17"
russh = "0.37.1"
russh-keys = "0.37.1"
thiserror = "1.0.40"

[build-dependencies]
shadow-rs = "0.23.0"
"#,
        );
        let t_test = Cursor::new(utils::compress_zlib(content.as_bytes()).unwrap());

        let mut deco = ReadBoxed::new(t_test, ObjectType::Blob, content.len());

        let _blob = Blob::new_from_read(&mut deco, content.len());

        assert_eq!(
            _blob.id.to_plain_str(),
            "b5e463cf00f754127a71c4ca09d53717672a93a2"
        );
    }

    #[test]
    fn test_new_from_file() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/objects/8a/b686eafeb1f44702738c8b0f24f2567c36da6d");
        let meta = Meta::new_from_file(source.to_str().unwrap()).unwrap();
        let blob = Blob::from_meta(meta);

        // Check Hash value
        assert_eq!(
            blob.id.to_plain_str(),
            "8ab686eafeb1f44702738c8b0f24f2567c36da6d"
        );
        // Check text content
        assert_eq!(blob.data[..], "Hello, World!\n".as_bytes().to_vec());
    }
}
