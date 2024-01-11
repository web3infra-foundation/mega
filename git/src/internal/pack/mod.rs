//!
//!
//!
//!
//!
use std::{path::PathBuf, sync::Arc};

use self::{
    cache::{ObjectCache, _Cache},
    header::EntryHeader,
};

use crate::hash::Hash;
use crate::internal::object::ObjectT;

mod cache;
pub mod counter;
mod cqueue;
pub mod decode;
pub mod delta;
pub mod encode;
mod header;
pub mod iterator;
pub mod preload;
/// ### Represents a Git pack file.
///  `head`: The file header, typically "PACK"<br>
/// `version`: The pack file version <br>
/// `number_of_objects` : The total number of objects in the pack <br>
/// `signature`:The pack file's hash signature <br>
/// `result`: decoded cache of pack objects <br>
/// `path` : The path to the pack file on disk
#[allow(unused)]
pub struct Pack {
    head: [u8; 4],
    version: u32,
    number_of_objects: usize,
    pub signature: Hash,
    path: PathBuf,
    cache: Box<dyn _Cache<T = Arc<dyn ObjectT>>>,
    //iterator: Option<iterator::EntriesIter<BR>>,
}

impl Default for Pack {
    fn default() -> Self {
        Self {
            head: Default::default(),
            version: Default::default(),
            number_of_objects: Default::default(),
            signature: Default::default(),
            path: Default::default(),
            cache: Box::new(ObjectCache::new(None)),
        }
    }
}

impl Pack {
    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn number_of_objects(&self) -> usize {
        self.number_of_objects
    }
    // pub fn get_cache(self) -> ObjectCache<Arc<dyn ObjectT>> {
    //     //self.cache
    // }
}

pub struct Entry {
    pub header: EntryHeader,
    pub decompressed_size: u64,
    pub offset: u64,
}

pub mod git_object_size {
    use std::io::{Read, Write};

    // Function to decode the size of a Git object from a reader
    pub fn decode<R: Read>(mut reader: R) -> std::io::Result<usize> {
        // Initialize the size and shift variables
        let mut size = 0;
        let mut shift = 0;

        // Buffer to hold the current byte
        let mut buffer = [0; 1];

        // Loop over the bytes from the reader
        while reader.read_exact(&mut buffer).is_ok() {
            // Get the current byte
            let byte = buffer[0];
            // Update the size by bitwise OR with the lower 7 bits of the byte, shifted left by the shift amount
            size |= ((byte & 0x7f) as usize) << shift;
            // If the highest bit of the byte is 0, break the loop
            if byte & 0x80 == 0 {
                break;
            }
            // Increase the shift amount by 7 for the next byte
            shift += 7;
        }

        // Return the decoded size
        Ok(size)
    }

    // Function to encode the size of a Git object and write it to a writer
    pub fn encode<W: Write>(mut writer: W, mut size: usize) -> std::io::Result<()> {
        // Buffer to hold the current byte
        let mut buffer = [0u8; 1];

        // Loop until the size is 0
        while size > 0 {
            // Get the lower 7 bits of the size
            buffer[0] = (size & 0x7f) as u8;
            // Shift the size right by 7 bits
            size >>= 7;
            // If there are more bits, set the highest bit of the byte
            if size > 0 {
                buffer[0] |= 0x80;
            }
            // Write the byte to the writer
            writer.write_all(&buffer)?;
        }

        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use std::io::Cursor;

        use crate::internal::pack::git_object_size::{decode, encode};
        #[test]
        fn test_decode() {
            let data = [0x82, 0x01];
            let cursor = Cursor::new(data);
            let result = decode(cursor).unwrap();
            assert_eq!(result, 130);
        }

        #[test]
        fn test_encode() {
            let size = 130;
            let mut data = Vec::new();
            encode(&mut data, size).unwrap();
            assert_eq!(data, [0x82, 0x01]);
        }
    }
}

#[cfg(test)]
mod tests {}
