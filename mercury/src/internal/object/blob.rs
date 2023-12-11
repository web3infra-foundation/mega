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

use bstr::ByteSlice;

use crate::hash::SHA1;
use crate::internal::object::ObjectTrait;
use crate::internal::object::types::ObjectType;


/// **The Blob Object**
///
#[allow(unused)]
#[derive(Eq, Debug, Clone)]
pub struct Blob {
    pub id: SHA1,
    pub size: usize,
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
    }
}

impl ObjectTrait for Blob {
    fn from_bytes(data: Vec<u8>) -> Self {
        let id = SHA1::new(&data);

        let header_offset = data.find_byte(0x20).unwrap();
        let header = &data[0..header_offset];
        assert_eq!(header, b"blob");

        let size_offset = data.find_byte(0x00).unwrap();
        let size = parse_size_from_bytes(&data[header_offset + 1..size_offset]).unwrap();

        let data = &data[size_offset + 1..].to_vec();
        assert_eq!(size, data.len());

        Blob { id, size, data: data.to_vec() }
    }

    fn get_type(&self) -> ObjectType {
        ObjectType::Blob
    }
}

fn parse_size_from_bytes(bytes: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
    let size_str = std::str::from_utf8(bytes)?;
    Ok(size_str.parse::<usize>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_from_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let size: usize = 12345;
        let size_bytes = size.to_string().as_bytes().to_vec();
        
        let parsed_size = parse_size_from_bytes(&size_bytes)?;

        assert_eq!(size, parsed_size);
        Ok(())
    }
}