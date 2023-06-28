//!
//! In the Git object binary model:
//!
//! - **Null** bytes are used as separators between the different fields to allow for efficient parsing
//! of the object.
//!
//!
//!
pub mod blob;
pub mod cache;
pub mod commit;
pub mod meta;
pub mod reader;
pub mod signature;
pub mod tag;
pub mod tree;
pub enum Object {
    BLOB(blob::Blob),
    TREE(tree::Tree),
    COMMIT(commit::Commit),
    TAG(tag::Tag),
}

pub trait ObjRead: Read + Seek + Send {}

use self::reader::ObjReader;
use crate::{hash::Hash, utils};
use sha1::Digest;
use std::{
    fmt::Display,
    io::{BufRead, Read, Seek},
};

use super::{pack::delta::DeltaReader, zlib::stream::inflate::ReadBoxed, ObjectType};
pub trait ObjectT: Send + Sync + Display {
    fn get_hash(&self) -> Hash;
    fn set_hash(&mut self, h: Hash);
    fn get_type(&self) -> ObjectType;
    fn new(mut input: ObjReader<impl Read + Seek + Send>) -> Self
    where
        Self: Sized,
    {
        //input.read_to_end( &mut content).unwrap();
        let read_size = input.glen();
        let mut content: Vec<u8> = Vec::with_capacity(read_size);
        utils::read_chars(&mut input, &mut content, read_size).unwrap();
        //Seek
        //input.seek_inner();

        let hash_str = input.finalize();
        let h = Hash::new_from_str(&hash_str);

        let mut re = Self::new_from_data(content);
        re.set_hash(h);
        re
    }
    fn new_from_read<R: BufRead>(read: &mut ReadBoxed<R>, size: usize) -> Self
    where
        Self: Sized,
    {
        let mut content: Vec<u8> = Vec::with_capacity(size);
        read.read_to_end(&mut content).unwrap();
        //TODO : hash
        let h = read.hash.clone();
        let hash_str = h.finalize();
        let mut result = Self::new_from_data(content);
        result.set_hash(Hash::new_from_str(&format!("{:x}", hash_str)));

        result
    }

    fn new_delta(read: &mut DeltaReader) -> Self
    where
        Self: Sized,
    {
        let mut content: Vec<u8> = Vec::with_capacity(read.len());
        read.read_to_end(&mut content).unwrap();
        //TODO : hash
        let h = read.hash.clone();
        let hash_str = h.finalize();
        let mut result = Self::new_from_data(content);
        result.set_hash(Hash::new_from_str(&format!("{:x}", hash_str)));
        result
    }
    fn get_raw(&self) -> &[u8];
    fn new_from_data(data: Vec<u8>) -> Self
    where
        Self: Sized;
}

#[cfg(test)]
mod tests {}
