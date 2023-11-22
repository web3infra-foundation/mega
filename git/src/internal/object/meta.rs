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
use bstr::ByteSlice;
use deflate::{write::ZlibEncoder, Compression};
use flate2::read::ZlibDecoder;

use crate::errors::GitError;
use crate::hash::Hash;
use crate::internal::ObjectType;

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
}

impl Meta {
    /// Create a new `Meta` struct from a Git object without data.
    #[allow(unused)]
    pub fn default(object_type: ObjectType) -> Self {
        Meta {
            object_type,
            id: Hash::default(),
            size: 0,
            data: vec![],
        }
    }

    /// A Git object in the Loose Format consists of two parts: the object header and the object data.
    /// The object header is stored in plain text format and contains the following information:
    ///
    /// - Object type: type of Git object the object is, such as a blob, tree, commit, or tag.
    /// - Object size: size of the object data in bytes.
    ///
    /// The object header has a space(`\x32`) between the object type and the object size.
    ///
    /// The object header is followed by a null byte (0x00) and then the object data.
    ///
    /// The object id is calculated from the object header and the object data.
    pub fn calculate_id(object_type: ObjectType, data: &Vec<u8>) -> Hash {
        let mut d: Vec<u8> = Vec::new();

        d.extend(object_type.to_data().unwrap());
        d.push(b' ');
        d.extend(data.len().to_string().as_bytes());
        d.push(b'\x00');
        d.extend(data);

        Hash::new(&d)
    }

    /// Create a new `Meta` struct from a Git object include object type and data.
    /// # Examples
    /// ```
    ///     use git::internal::object::meta::Meta;
    ///     use git::internal::ObjectType;
    ///
    ///     let meta = Meta::new_from_data_with_object_type(ObjectType::Blob, vec![98, 108, 111, 98, 32, 49, 52, 0, 72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33, 10]);
    ///     assert_eq!(meta.object_type, ObjectType::Blob);
    //      assert_eq!(meta.id.to_plain_str(), "8ab686eafeb1f44702738c8b0f24f2567c36da6d");
    //      assert_eq!(meta.size, 14);
    //      assert_eq!(meta.delta_header, vec![]);
    /// ```
    #[allow(unused)]
    pub fn new_from_data_with_object_type(object_type: ObjectType, data: Vec<u8>) -> Self {
        Meta {
            object_type,
            id: Meta::calculate_id(object_type, &data),
            size: data.len(),
            data,
        }
    }

    #[allow(unused)]
    pub fn new_from_data(bytes: Vec<u8>) -> Result<Self, GitError> {
        let type_index = bytes.find_byte(0x20).unwrap();
        let type_object = &bytes[0..type_index];

        let size_index = bytes.find_byte(0x00).unwrap();
        let data = bytes[size_index + 1..].to_vec();

        match String::from_utf8(type_object.to_vec()).unwrap().as_str() {
            "blob" => Ok(Meta::new_from_data_with_object_type(ObjectType::Blob, data)),
            "tree" => Ok(Meta::new_from_data_with_object_type(ObjectType::Tree, data)),
            "commit" => Ok(Meta::new_from_data_with_object_type(
                ObjectType::Commit,
                data,
            )),
            "tag" => Ok(Meta::new_from_data_with_object_type(ObjectType::Tag, data)),
            _ => Err(GitError::InvalidObjectType(
                String::from_utf8(type_object.to_vec()).unwrap(),
            )),
        }
    }

    /// # Attention
    /// In the ASCII character set, the character corresponding 10(hex: 0x0A) is the line feed (LF)
    /// character, which is commonly used as a symbol for a new line in text files. The LF character
    /// is represented by the hexadecimal value of 0x0A in ASCII. The way that new lines are
    /// represented in text files can vary across different operating systems and programming
    /// languages. In Linux and Unix systems, LF is commonly used to represent new lines in text
    /// files, while in Windows systems, a combination of carriage return (CR) and LF ("\r\n") is
    /// commonly used.
    #[allow(unused)]
    pub fn new_from_file(path: &str) -> Result<Self, GitError> {
        let file = File::open(path).unwrap();
        let mut reader = BufReader::new(file);

        let mut decoder = ZlibDecoder::new(reader);
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded).unwrap();

        Self::new_from_data(decoded)
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
    /// TODO: Add a overwrite flag to control whether to overwrite the existing file.
    /// TODO: Add a file path parameter to control where to store the file without flow Git store spec.
    #[allow(unused)]
    pub fn to_file(&self, root: &str) -> Result<PathBuf, GitError> {
        // e is a ZlibEncoder, which is a wrapper around a Writer that compresses the data written to
        let mut e = ZlibEncoder::new(Vec::new(), Compression::Default);

        // Write the object type to the encoder
        // Object Type + Space + Object Size + \0 + Object Data
        e.write_all(&self.object_type.to_data().unwrap());
        e.write(&[b' ']);
        e.write(self.size.to_string().as_bytes());
        e.write(&[b'\0']);
        e.write_all(&self.data)
            .with_context(|| format!("Failed to write to encoder: {}", self.id.to_plain_str()));
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

        Ok(path)
    }
}

#[cfg(test)]
mod tests {

    use std::env;
    use std::fs::remove_file;
    use std::path::PathBuf;

    use crate::internal::object::meta::Meta;
    use crate::internal::ObjectType;

    #[test]
    fn test_meta_default() {
        let meta = Meta::default(ObjectType::Blob);

        assert_eq!(meta.object_type, ObjectType::Blob);
        assert_eq!(meta.size, 0);
    }

    #[test]
    fn test_new_from_data_with_object_type() {
        let meta = Meta::new_from_data_with_object_type(
            ObjectType::Blob,
            "Hello, World!".as_bytes().to_vec(),
        );

        assert_eq!(meta.object_type, ObjectType::Blob);
        assert_eq!(meta.size, 13);
        assert_eq!(
            meta.id.to_plain_str(),
            "b45ef6fec89518d314f546fd6c3025367b721684"
        );
    }

    #[test]
    fn test_to_folder_name() {
        let meta = Meta::new_from_data_with_object_type(
            ObjectType::Blob,
            "Hello, World!".as_bytes().to_vec(),
        );

        assert_eq!(meta.to_folder_name(), "b4");
    }

    #[test]
    fn test_to_file_name() {
        let meta = Meta::new_from_data_with_object_type(
            ObjectType::Blob,
            "Hello, World!".as_bytes().to_vec(),
        );

        assert_eq!(
            meta.to_file_name(),
            "5ef6fec89518d314f546fd6c3025367b721684"
        );
    }

    #[test]
    fn test_new_from_file() {
        // "Hello, World!" is [72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33] without
        // line feed(LF), which is a control character used to represent the end of a line of text and the
        // beginning of a new line. The LF character is commonly used in Unix and Unix-like operating
        // systems (including Linux and macOS) as the standard end-of-line marker in text files
        //
        // So, When read a file include the same content "Hello, World!", it's different SHA-1
        // value calculated.
        //
        // The object is stored in the tests/data/objects/8a/b686eafeb1f44702738c8b0f24f2567c36da6d
        // In Linux and Unix systems, LF is commonly used to represent new lines in text files, while
        // in Windows systems, a combination of carriage return (CR) and LF ("\r\n") is commonly used.
        //
        // "Hello, World!" is [72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33]
        // "Hello, World!" read from file is [72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33, 10]
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/objects/8a/b686eafeb1f44702738c8b0f24f2567c36da6d");

        let meta = Meta::new_from_file(source.as_path().to_str().unwrap()).unwrap();

        assert_eq!(meta.object_type, ObjectType::Blob);
        assert_eq!(meta.size, 14);
        assert_eq!(
            meta.id.to_plain_str(),
            "8ab686eafeb1f44702738c8b0f24f2567c36da6d"
        );
    }

    #[test]
    fn test_write_to_file() {
        let source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());

        let mut source_file = source;
        source_file.push("tests/data/objects/8a/b686eafeb1f44702738c8b0f24f2567c36da6d");
        let m = Meta::new_from_file(source_file.as_path().to_str().unwrap()).unwrap();

        let mut dest = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        dest.push("tests/objects");

        let mut dest_file = dest.clone();
        dest_file.push("8a/b686eafeb1f44702738c8b0f24f2567c36da6d");
        if dest_file.exists() {
            remove_file(dest_file.as_path().to_str().unwrap()).unwrap();
        }

        let file = m.to_file(dest.as_path().to_str().unwrap()).unwrap();

        assert!(file.exists());
    }
}
