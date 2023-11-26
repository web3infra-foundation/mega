//!
//! 
//! 
//! 
//! 
use std::io::{BufRead, Read};

/// The [`ObjectT`] Trait is for the Blob、Commit、Tree and Tag Structs , which are four common object
/// of Git object . In that case, the four kinds of object can be store in same `Arc<dyn ObjectT>`.
///
/// This trait  receive a "Reader" to generate the target object. We now have two kind of "Reader":
/// 
/// 1. ReadBoxed. Input the zlib stream of four kinds of objects data stream. The Object should be the 
/// base objects ,that is ,"Blob、Commit、Tree and Tag". After read, Output Object will auto compute hash 
/// value while call the "read" method.
/// 2. DeltaReader. To deal with the DELTA object store in the pack file,including the Ref Delta Object 
/// and the Offset Delta Object. Its' input "read" is always the `ReadBoxed`, cause the delta data is also 
/// the zlib stream, which should also be unzip.
pub trait ObjectT: Send + Sync + Display {
    /// Generate a new Object from a `ReadBoxed<BufRead>`.
    /// the input size,is only for new a vec with directive space allocation
    /// the Input data stream and  Output object should be plain base object.
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
}