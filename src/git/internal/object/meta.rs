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
    pub fn to_folder_name(&self) -> String {
        self.id.to_plain_str()[..2].to_string()
    }

    #[allow(unused)]
    pub fn to_file_name(&self) -> String {
        self.id.to_plain_str()[2..].to_string()
    }

    /// Write the object to the file system with crate folder at the same time.
    /// This function can create a “loose” object format, which is the default format for storing
    /// objects in the Git database.
    ///
    /// Git objects in the Loose Format are stored in the .git/objects directory of the Git
    /// repository, with each object stored in a file named after its SHA-1 checksum.
    /// The Loose Format is suitable for storing a small number of Git objects, or in cases where Git
    /// objects do not need to be frequently read or modified.
    ///
    /// A Git object in the Loose Format consists of two parts: the object header and the object data.
    /// The object header is stored in plain text format in the first few lines of the object file,
    /// and contains the following information:
    ///
    /// - Object type: type of Git object the object is, such as a Blob, Tree, Commit, or Tag.
    /// - Object size: size of the object data in bytes.
    /// - Object data: the actual content.
    ///
    /// |Object Type|Object Size|Object Data|
    /// |-----------|-----------|-----------|
    /// |blob|13 bytes|"Hello, World!"|
    ///
    #[allow(unused)]
    pub fn  loose_2file(&self, root: &str) -> Result<String, GitError> {
        // e is a ZlibEncoder, which is a wrapper around a Writer that compresses the data written to
        let mut e = ZlibEncoder::new(Vec::new(), Compression::Default);

        // Write the object type to the encoder
        // Object Type + Space + Object Size + \0 + Object Data
        e.write_all(&self.object_type.to_bytes().unwrap());
        e.write(&[b' ']);
        e.write(self.size.to_string().as_bytes());
        e.write(&[b'\0']);
        e.write_all(&self.data).with_context(
            || format!("Failed to write to encoder: {}", self.id.to_plain_str()));
        let c = e.finish().unwrap();

        // Create the folder
        let mut path = PathBuf::from(root);
        path.push(&self.to_folder_name());
        create_dir_all(&path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))
            .unwrap();

        // Add filename to the path
        path.push(&self.to_file_name());

        // Create the file
        let mut file = File::create(&path)
            .with_context(|| format!("Failed to create file: {}", path.display()))
            .unwrap();

        // Write the compressed data to the file
        file.write_all(&c)
            .with_context(|| format!("Failed to write to file: {}", path.display()))
            .unwrap();

        Ok(path.to_str().unwrap().to_string())
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
    pub fn pack_2vec(&self) -> Result<Vec<u8>, GitError> {
        let mut data =
            // `<< 4` This left-shifts the result of type2number() by 4 bits, effectively multiplying
            // it by 16. This leaves the 4 least significant bits of the byte set to 0.
            // `0x80 | ...` This sets the most significant bit of the byte to 1, indicating that the
            // byte represents a Git object. Specifically, the value 0x80 has its most significant
            // bit set to 1, and all other bits set to 0.
            vec![(0x80 | (self.object_type.type2number() << 4))
                // + (self.size & 0x0f) as u8: This sets the 4 least significant bits of the byte
                // to the low-order 4 bits of the object's size. Specifically, self.size & 0x0f
                // performs a bitwise AND operation between self.size and the hexadecimal value 0x0f,
                // which results in the low-order 4 bits of self.size. This value is then cast to a
                // u8 byte and added to the byte constructed in steps 2 and 3.
                + (self.size & 0x0f) as u8];

        let mut s = self.size >> 4;

        if s > 0 {
            while s > 0 {
                if s >> 7 > 0 {
                    data.push((0x80 | s) as u8);
                    s >>= 7;
                } else {
                    data.push((s) as u8);
                    break;
                }
            }
        } else {
            data.push(0);
        }

        match self.object_type {
            ObjectType::OffsetDelta => {
                data.append(&mut self.delta_header.clone());
            }
            ObjectType::HashDelta => {
                data.append(&mut self.delta_header.clone());
            }
            _ => {}
        }

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::Default);
        encoder.write_all(&self.data).expect("Write error!");
        data.append(&mut encoder.finish().expect("Failed to finish compression!"));

        Ok(data)
    }
}

mod tests {
    #[test]
    fn test_meta_new() {
        use crate::git::internal::ObjectType;

        let meta = super::Meta::new(ObjectType::Blob,0,
                                    vec![], vec![]);

        assert_eq!(meta.object_type, ObjectType::Blob);
        assert_eq!(meta.id.to_plain_str(), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(meta.size, 0);
        assert_eq!(meta.data.len(), 0);
        assert_eq!(meta.delta_header.len(), 0);
    }

    #[test]
    fn test_new_from_data() {
        use crate::git::internal::ObjectType;

        let meta = super::Meta::new_from_data(ObjectType::Blob,
                                              "Hello, World!".as_bytes().to_vec());

        assert_eq!(meta.object_type, ObjectType::Blob);
        assert_eq!(meta.size, 13);
        assert_eq!(meta.id.to_plain_str(), "0a0a9f2a6772942557ab5355d76af442f8f65e01");
    }

    #[test]
    fn test_new_from_data_with_delta() {
        use crate::git::internal::ObjectType;

        let meta = super::Meta::new_from_data_and_delta_header(
            ObjectType::OffsetDelta,
            "Hello, World!".as_bytes().to_vec(),
            vec![0x00, 0x00, 0x00, 0x00]);

        assert_eq!(meta.object_type, ObjectType::OffsetDelta);
        assert_eq!(meta.size, 13);
        assert_eq!(meta.id.to_plain_str(), "0a0a9f2a6772942557ab5355d76af442f8f65e01");
        assert_eq!(meta.delta_header, vec![0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_to_folder_name() {
        use crate::git::internal::ObjectType;

        let meta = super::Meta::new_from_data(ObjectType::Blob,
                                              "Hello, World!".as_bytes().to_vec());

        assert_eq!(meta.to_folder_name(), "0a");
    }

    #[test]
    fn test_to_file_name() {
        use crate::git::internal::ObjectType;

        let meta = super::Meta::new_from_data(ObjectType::Blob,
                                              "Hello, World!".as_bytes().to_vec());

        assert_eq!(meta.to_file_name(), "0a9f2a6772942557ab5355d76af442f8f65e01");
    }
    
    #[test]
    fn test_loose_2file() {
        //8ab686eafeb1f44702738c8b0f24f2567c36da6d

        use crate::git::internal::ObjectType;
        use std::env;
        use std::path::PathBuf;
        use std::fs::remove_file;

        let mut project = PathBuf::from(env::current_dir().unwrap());
        project.push("tests/objects");

        let mut dest = PathBuf::from(env::current_dir().unwrap());
        dest.push("tests/objects/0a/0a9f2a6772942557ab5355d76af442f8f65e01");
        if dest.exists() {
            remove_file(dest.as_path().to_str().unwrap()).unwrap();
        }

        let meta = super::Meta::new_from_data(ObjectType::Blob,
                                              "Hello, World!".as_bytes().to_vec());

        let file = meta.loose_2file(project.as_path().to_str().unwrap()).unwrap();

        assert_eq!(file, dest.as_path().to_str().unwrap());
    }

    #[test]
    fn test_new_from_file() {
        use crate::git::internal::ObjectType;
        use std::env;
        use std::path::PathBuf;
        use std::fs::remove_file;

        //1c03657b57ce1ba77b9243e48687e9f7183acd86

        let mut project = PathBuf::from(env::current_dir().unwrap());
        project.push("tests/objects");

        let mut dest = PathBuf::from(env::current_dir().unwrap());
        dest.push("tests/objects/89/b06a0467f41bb25165467f01a8fc86d94436e8");
        if dest.exists() {
            remove_file(dest.as_path().to_str().unwrap()).unwrap();
        }

        let meta = super::Meta::new_from_data(ObjectType::Blob,
                                              "Hello, Rust!".as_bytes().to_vec());

        let file = meta.loose_2file(project.as_path().to_str().unwrap()).unwrap();

        let meta2 = super::Meta::new_from_file(file.as_str()).unwrap();

        assert_eq!(meta2.object_type, ObjectType::Blob);
        assert_eq!(meta2.size, 12);
        assert_eq!(meta2.id.to_plain_str(), "89b06a0467f41bb25165467f01a8fc86d94436e8");
    }
}