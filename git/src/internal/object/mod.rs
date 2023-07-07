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
pub mod signature;
pub mod tag;
pub mod tree;
use self::meta::Meta;
use super::{pack::delta::DeltaReader, zlib::stream::inflate::ReadBoxed, ObjectType};
use crate::hash::Hash;
use sha1::Digest;
use std::{
    fmt::Display,
    io::{BufRead, Read},
};

/// The [`ObjectT`] Trait is for the Blob、Commit、Tree and Tag Structs , which are four common object
/// of git object . In that case, the four kinds of object can be store in same `Arc<dyn ObjectT>`.
///
/// This trait  receive a "Reader" to generate the target object. We now have two kind of "Reader":
/// 1. ReadBoxed. Input the zlib stream of four kinds of objects data stream. The Object should be the base objects ,that is ,"Blob、Commit、Tree and Tag". After read, Output Object will auto compute hash value while call the "read" method.
/// 2. DeltaReader. To deal with the DELTA object store in the pack file,including the Ref Delta Object and the Offset Delta Object. Its' input "read" is always the `ReadBoxed`, cause the delta data is also the zlib stream, which should also be unzip.
pub trait ObjectT: Send + Sync + Display {
    /// Get the hash value .
    fn get_hash(&self) -> Hash;
    /// Set the hash value for object .
    fn set_hash(&mut self, h: Hash);
    /// Get Object Type ,see [`ObjectType`]
    fn get_type(&self) -> ObjectType;

    /// Generate a new Object from a `ReadBoxed<BufRead>`.
    /// the input size,is only for new a vec with directive space allocation
    /// the Input data stream and  Output object should be plain base object .
    fn new_from_read<R: BufRead>(read: &mut ReadBoxed<R>, size: usize) -> Self
    where
        Self: Sized,
    {
        let mut content: Vec<u8> = Vec::with_capacity(size);
        read.read_to_end(&mut content).unwrap();
        let h = read.hash.clone();
        let hash_str = h.finalize();
        let mut result = Self::new_from_data(content);
        result.set_hash(Hash::new_from_str(&format!("{:x}", hash_str)));

        result
    }
    /// Generate a new Object from DeltaReader
    /// Output Object should be decoded from a delta object data stream .
    fn new_delta(read: &mut DeltaReader) -> Self
    where
        Self: Sized,
    {
        let mut content: Vec<u8> = Vec::with_capacity(read.len());
        read.read_to_end(&mut content).unwrap();
        let h = read.hash.clone();
        let hash_str = h.finalize();
        let mut result = Self::new_from_data(content);
        result.set_hash(Hash::new_from_str(&format!("{:x}", hash_str)));
        result
    }

    /// Get raw data from the Object.
    fn get_raw(&self) -> Vec<u8>;
    fn new_from_data(data: Vec<u8>) -> Self
    where
        Self: Sized;

    fn from_meta(meta: Meta) -> Self
    where
        Self: Sized,
    {
        let mut r = Self::new_from_data(meta.data);
        r.set_hash(meta.id);
        r
    }
}

#[cfg(test)]
mod tests {}
