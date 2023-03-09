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

use crate::git::errors::GitError;
use crate::git::internal::object::meta::Meta;
use crate::git::internal::object::tree::{TreeItem, TreeItemMode};
use crate::git::internal::ObjectType;

/// **The Blob Object**
///
/// # Attention
/// 1. The blob content is stored in the Meta object, so the Blob object only stores the Meta object.
/// 2. When the object saving to the disk, the Git use zip compression algorithm to compress.
#[allow(unused)]
#[derive(Eq, Debug, Clone)]
pub struct Blob {
    pub meta: Meta,
}

impl PartialEq for Blob {
    /// The Blob object is equal to another Blob object if their IDs are equal.
    fn eq(&self, other: &Self) -> bool {
        self.meta.id == other.meta.id
    }
}

impl Display for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Size:{}", self.meta.size).unwrap();
        writeln!(f, "Type:{}", self.meta.object_type).unwrap();
        writeln!(f, "{:?}", str::from_utf8(&self.meta.data))
    }
}

impl Blob {
    /// Create a new Blob object from a Meta object.
    #[allow(unused)]
    pub fn new_from_meta(meta: Meta) -> Result<Blob, GitError> {
        if meta.object_type != ObjectType::Blob {
            return Err(GitError::InvalidBlobObject(meta.object_type.to_string()));
        }

        Ok(Self { meta })
    }

    /// Create a new Blob object from a data.
    #[allow(unused)]
    pub fn new_from_data(data: Vec<u8>) -> Self {
        Self {
            meta: Meta::new_from_data(ObjectType::Blob, data),
        }
    }

    /// Create a new Blob object from a file.
    #[allow(unused)]
    pub fn new_from_file(path: &str) -> Result<Self, GitError> {
        Ok(Self {
            meta: Meta::new_from_file(path)?,
        })
    }

    /// Write the Blob object to a file with the given root path.
    /// The file path is the root path + ID[..2] + ID[2..]
    #[allow(unused)]
    pub fn write_to_file(&self, path: &str) -> Result<String, GitError> {
        self.meta.write_to_file(path)
    }

    /// Generate a tree item string for the Blob object.
    #[allow(unused)]
    pub fn generate_tree_item(&self, filename: &str) -> Result<TreeItem, GitError> {
        Ok(
            TreeItem {
                mode: TreeItemMode::Blob,
                id: self.meta.id,
                name: filename.to_string(),
            }
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_new_from_meta() {
        use crate::git::internal::object::meta::Meta;
        use crate::git::internal::object::blob::Blob;
        use crate::git::internal::ObjectType;

        let meta = Meta::new_from_data(ObjectType::Blob, "Hello, World!".as_bytes().to_vec());
        let blob = Blob::new_from_meta(meta).unwrap();

        assert_eq!(blob.meta.data, "Hello, World!".as_bytes().to_vec());
    }

    #[test]
    fn test_new_from_data() {
        use crate::git::internal::object::blob::Blob;

        let blob = Blob::new_from_data("Hello, World!".as_bytes().to_vec());

        assert_eq!(blob.meta.id.to_plain_str(), "b45ef6fec89518d314f546fd6c3025367b721684");
        assert_eq!(blob.meta.data, "Hello, World!".as_bytes().to_vec());
    }

    #[test]
    fn test_new_from_file() {
        use std::env;
        use std::path::PathBuf;

        use crate::git::internal::object::blob::Blob;

        let mut source = PathBuf::from(env::current_dir().unwrap());
        source.push("tests/data/objects/8a/b686eafeb1f44702738c8b0f24f2567c36da6d");

        let blob = Blob::new_from_file(source.to_str().unwrap()).unwrap();

        assert_eq!(blob.meta.id.to_plain_str(), "8ab686eafeb1f44702738c8b0f24f2567c36da6d");
        // Remove the last byte of the data, because the last byte is a newline character.
        assert_eq!(blob.meta.data[..blob.meta.size - 1], "Hello, World!".as_bytes().to_vec());
    }

    #[test]
    fn test_write_to_file() {
        use std::env;
        use std::path::PathBuf;
        use std::fs::remove_file;

        use crate::git::internal::object::blob::Blob;

        let mut source = PathBuf::from(env::current_dir().unwrap());
        source.push("tests/data/objects/8a/b686eafeb1f44702738c8b0f24f2567c36da6d");
        let blob = Blob::new_from_file(source.to_str().unwrap()).unwrap();

        let mut dest_file = PathBuf::from(env::current_dir().unwrap());
        dest_file.push("tests/objects/8a/b686eafeb1f44702738c8b0f24f2567c36da6d");
        if dest_file.exists() {
            remove_file(dest_file.as_path().to_str().unwrap()).unwrap();
        }

        let mut dest = PathBuf::from(env::current_dir().unwrap());
        dest.push("tests/objects");
        let file = blob.write_to_file(dest.as_path().to_str().unwrap()).unwrap();

        dest.push("8a");
        dest.push("b686eafeb1f44702738c8b0f24f2567c36da6d");
        assert_eq!(file, dest.as_path().to_str().unwrap());
    }

    #[test]
    fn test_generate_tree_item() {
        use crate::git::internal::object::blob::Blob;
        use crate::git::internal::object::tree::TreeItemMode;

        let blob = Blob::new_from_data("Hello, World!".as_bytes().to_vec());
        let tree_item = blob.generate_tree_item("hello-world").unwrap();

        assert_eq!(tree_item.mode, TreeItemMode::Blob);
        assert_eq!(tree_item.id.to_plain_str(), "b45ef6fec89518d314f546fd6c3025367b721684");
        assert_eq!(tree_item.name, "hello-world");
    }
}