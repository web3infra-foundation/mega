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
//! This will output an SHA-1 hash, which is the ID of the newly created blob object.
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

use crate::errors::GitError;
use crate::hash::SHA1;
use crate::internal::object::types::ObjectType;
use crate::internal::object::ObjectTrait;

/// **The Blob Object**
#[derive(Eq, Debug, Clone)]
pub struct Blob {
    pub id: SHA1,
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
        writeln!(f, "Type: Blob").unwrap();
        writeln!(f, "Size: {}", self.data.len())
    }
}

impl ObjectTrait for Blob {
    /// Creates a new object from a byte slice.
    fn from_bytes(data: &[u8], hash: SHA1) -> Result<Self, GitError>
    where
        Self: Sized,
    {
        Ok(Blob {
            id: hash,
            data: data.to_vec(),
        })
    }

    /// Returns the Blob type
    fn get_type(&self) -> ObjectType {
        ObjectType::Blob
    }

    fn get_size(&self) -> usize {
        self.data.len()
    }

    fn to_data(&self) -> Result<Vec<u8>, GitError> {
        Ok(self.data.clone())
    }
}

impl Blob {
    /// Create a new Blob object from the given content string.
    /// - This is a convenience method for creating a Blob from a string.
    /// - It converts the string to bytes and then calls `from_content_bytes`.
    pub fn from_content(content: &str) -> Self {
        let content = content.as_bytes().to_vec();
        Blob::from_content_bytes(content)
    }

    /// Create a new Blob object from the given content bytes.
    /// - some file content can't be represented as a string (UTF-8), so we need to use bytes.
    pub fn from_content_bytes(content: Vec<u8>) -> Self {
        Blob {
            // Calculate the SHA1 hash from the type and content
            id: SHA1::from_type_and_data(ObjectType::Blob, &content),
            data: content,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_blob_from_content() {
        let content = "Hello, world!";
        let blob = Blob::from_content(content);
        assert_eq!(
            blob.id.to_string(),
            "5dd01c177f5d7d1be5346a5bc18a569a7410c2ef"
        );
    }
}
