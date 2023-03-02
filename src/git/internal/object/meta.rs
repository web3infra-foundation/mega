//! By providing this metadata information, the Meta struct enables us to manipulate Git objects
//! more efficiently and easily in Rust programs. For example, we can use the object_type field to
//! determine the type of a Git object, and the id field to identify the object's location in the
//! Git database. We can also use the size field to check the size of the object's data, and the
//! data field to access the object's content.
//!
use std::fs::{create_dir_all, File};
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::Context;
use deflate::{Compression, write::ZlibEncoder};
use flate2::read::ZlibDecoder;
use bstr::ByteSlice;

use crate::git::errors::GitError;
use crate::git::hash::Hash;
use crate::git::internal::ObjectType;

/// `Meta` struct that provides metadata information for Git objects, including object type,
/// object ID (represented by a Hash struct), object size, object data, and delta header.
///
/// * `object_type`: An `ObjectType` value that represents the type of the Git object.
/// * `id`: A `Hash` struct that represents the SHA-1 hash of the Git object.
/// * `size`: An `usize` value that represents the size of the Git object's data in bytes.
/// * `data`: A `byte` array that represents the data of the Git object.
/// * `delta_header`: A byte array that represents the header of a Git delta object, used for
/// representing changes between two Git objects. Additionally, the delta_header field is useful
/// for Git objects that represent changes between two other objects, such as delta-encoded blobs
/// or commits. By storing the delta header separately from the object data, we can easily apply
/// the changes to the base object and obtain the resulting object.

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Meta {
    pub object_type: ObjectType,
    pub id: Hash,
    pub size: usize,
    pub data: Vec<u8>,
    pub delta_header: Vec<u8>,
}

impl Meta {
    /// Create a new `Meta` struct.
    /// # Examples
    /// ```
    ///     let meta = Meta::new(ObjectType::Blob, 0, vec![], vec![]);
    ///     assert_eq!(meta.object_type, ObjectType::Blob);
    ///     assert_eq!(meta.id, Hash::new());
    ///     assert_eq!(meta.size, 0);
    ///     assert_eq!(meta.data, vec![]);
    ///     assert_eq!(meta.delta_header, vec![]);
    /// ```
    #[allow(unused)]
    pub fn new(
        object_type: ObjectType,
        size: usize,
        data: Vec<u8>,
        delta_header: Vec<u8>,
    ) -> Self {
        Meta {
            object_type,
            id: Hash::new(&data),
            size,
            data,
            delta_header,
        }
    }

    #[allow(unused)]
    pub fn new_from_data(object_type: ObjectType, data: Vec<u8>) -> Self {
        Meta {
            object_type,
            id: Hash::new(&data),
            size: data.len(),
            data,
            delta_header: vec![],
        }
    }

    #[allow(unused)]
    pub fn new_from_data_and_delta_header(
        object_type: ObjectType,
        data: Vec<u8>,
        delta_header: Vec<u8>,
    ) -> Self {
        Meta {
            object_type,
            id: Hash::new(&data),
            size: data.len(),
            data,
            delta_header,
        }
    }

    #[allow(unused)]
    pub fn new_from_file(path: &str) -> Result<Self, GitError> {
        let file = File::open(path).unwrap();
        let mut reader = BufReader::new(file);

        let mut decoder = ZlibDecoder::new(reader);
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded).unwrap();

        let type_index = decoded.find_byte(0x20).unwrap();
        let t = &decoded[0..type_index];

        let size_index = decoded.find_byte(0x00).unwrap();
        let size = decoded[type_index + 1..size_index]
            .iter()
            .copied()
            .map(|x| x as char)
            .collect::<String>()
            .parse::<usize>()
            .unwrap();

        let mut data = decoded[size_index + 1..].to_vec();

        match String::from_utf8(t.to_vec()).unwrap().as_str() {
            "blob" => Ok(Meta::new_from_data(ObjectType::Blob, data)),
            "tree" => Ok(Meta::new_from_data(ObjectType::Tree, data)),
            "commit" => Ok(Meta::new_from_data(ObjectType::Commit, data)),
            "tag" => Ok(Meta::new_from_data(ObjectType::Tag, data)),
            _ => Err(GitError::InvalidObjectType(
                String::from_utf8(t.to_vec()).unwrap(),
            )),
        }
    }

    #[allow(unused)]
    pub fn to_folder_name(&self) -> String {
        self.id.to_plain_str()[..2].to_string()
    }

    #[allow(unused)]
    pub fn to_file_name(&self) -> String {
        self.id.to_plain_str()[2..].to_string()
    }

    #[allow(unused)]
    pub fn convert_to_vec(&self) -> Result<Vec<u8>, GitError> {
        let mut compressed_data =
            vec![(0x80 | (self.object_type.type2number() << 4)) + (self.size & 0x0f) as u8];

        let mut _size = self.size >> 4;

        if _size > 0 {
            while _size > 0 {
                if _size >> 7 > 0 {
                    compressed_data.push((0x80 | _size) as u8);
                    _size >>= 7;
                } else {
                    compressed_data.push((_size) as u8);
                    break;
                }
            }
        } else {
            compressed_data.push(0);
        }

        match self.object_type {
            ObjectType::OffsetDelta => {
                compressed_data.append(&mut self.delta_header.clone());
            }
            ObjectType::HashDelta => {
                compressed_data.append(&mut self.delta_header.clone());
            }
            _ => {}
        }

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::Default);
        encoder.write_all(&self.data).expect("Write error!");
        compressed_data.append(&mut encoder.finish().expect("Failed to finish compression!"));

        Ok(compressed_data)
    }

    /// Write the object to the file system with crate folder at the same time.
    /// This function can create a “loose” object format, which is the default format for storing
    /// objects in the Git database.
    /// # Examples
    /// ```
    /// ```
    #[allow(unused)]
    pub fn  write_to_file(&self, path: &str) -> Result<String, GitError> {
        let compressed_data = self.convert_to_vec()?;

        let mut path = PathBuf::from(path);
        path.push(&self.to_folder_name());
        create_dir_all(&path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))
            .unwrap();

        path.push(&self.to_file_name());

        let mut file = File::create(&path)
            .with_context(|| format!("Failed to create file: {}", path.display()))
            .unwrap();

        file.write_all(&compressed_data)
            .with_context(|| format!("Failed to write to file: {}", path.display()))
            .unwrap();

        Ok(path.to_str().unwrap().to_string())
    }
}

mod tests {
    #[test]
    fn test_meta_new() {
        use crate::git::internal::ObjectType;

        let meta = super::Meta::new(ObjectType::Blob,0,
                                    vec![], vec![]);

        assert_eq!(meta.object_type, ObjectType::Blob);
        // The empty vec![] SHA-1 hash is `da39a3ee5e6b4b0d3255bfef95601890afd80709`
        assert_eq!(meta.id.to_plain_str(), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(meta.size, 0);
        assert_eq!(meta.data.len(), 0);
        assert_eq!(meta.delta_header.len(), 0);
    }
}