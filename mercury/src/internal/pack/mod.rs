//! 
//! ## Reference
//! 1. Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
//!
// use std::sync::Arc;
use std::io::{self, Read, Seek};

use common::utils;

use crate::errors::GitError;

///
/// 
/// 
#[allow(unused)]
pub struct Pack {
    pub number: usize,
}


/// 
/// 
impl Pack {
    /// Check the Header of the Pack include the **"PACK" head** , **Version Number** and  **Number of the Objects**
    /// and return the number of the objects
    pub fn check_header(pack: &mut impl Read) -> Result<u32, GitError> {
        // Get the Pack Head 4 bytes, which should be the "PACK"
        let magic = utils::read_bytes(pack).unwrap();

        if magic != *b"PACK" {
            return Err(GitError::InvalidPackHeader(format!(
                "{},{},{},{}",
                magic[0], magic[1], magic[2], magic[3]
            )));
        }

        // 4-byte version number (network byte order): Git currently accepts version number 2 or 3 but generates version 2.
        let version = u32::from_be_bytes(utils::read_bytes(pack).unwrap());
        if version != 2 {
            return Err(GitError::InvalidPackFile(format!(
                "Version Number is {}, not 2",
                version
            )));
        }

        // 4-byte number of objects contained in the pack (network byte order)
        // Observation: we cannot have more than 4G versions ;-) and more than 4G objects in a pack.
        let object_num = u32::from_be_bytes(utils::read_bytes(pack).unwrap());
        
        Ok(object_num)
    }

    /// (undeltified representation)
    /// n-byte type and length (3-bit type, (n-1)*7+4-bit length)
    /// compressed data
    /// 
    /// n-byte type and length (3-bit type, (n-1)*7+4-bit length)
    /// base object name if OBJ_REF_DELTA or a negative relative
    /// offset from the delta object's position in the pack if this
    /// is an OBJ_OFS_DELTA object
    /// compressed delta data
    /// 
    pub fn decode_pack_object(&mut self, _pack: &mut (impl Read + Seek), _offset: &mut usize) -> Result<usize, GitError> {
        Ok(0)
    }

    ///
    /// 
    /// 
    pub fn decode(&mut self, pack: &mut (impl Read + Seek)) -> Result<(), GitError> {
        let object_num = Pack::check_header(pack).unwrap();
        self.number = object_num as usize;

        let mut offset: usize = 12;
        let mut i = 1;

        while i < object_num {
            let _ = self.decode_pack_object(pack, &mut offset).unwrap();

            i += 1;
        }

        Ok(())
    }

}

/// Reads a byte from the given stream and checks if there are more bytes to continue reading.
///
/// The return value includes two parts: an unsigned integer formed by the first 7 bits of the byte,
/// and a boolean value indicating whether more bytes need to be read.
///
/// # Parameters
/// * `stream`: The stream from which the byte is read.
///
/// # Returns
/// Returns an `io::Result` containing a tuple. The first element is the value of the first 7 bits,
/// and the second element is a boolean indicating whether more bytes need to be read.
///
#[allow(unused)]
fn read_byte_and_check_continuation<R: Read>(stream: &mut R) -> io::Result<(u8, bool)> {
    // Create a buffer for a single byte
    let mut bytes = [0; 1];

    // Read exactly one byte from the stream into the buffer
    stream.read_exact(&mut bytes)?;

    // Extract the byte from the buffer
    let byte = bytes[0];

    // Extract the first 7 bits of the byte
    let value = byte & 0b0111_1111;

    // Check if the most significant bit (8th bit) is set, indicating more bytes to follow
    let msb = byte >= 128;

    // Return the extracted value and the continuation flag
    Ok((value, msb))
}

/// Reads bytes from the stream and parses the first byte for type and size.
/// Subsequent bytes are read as size bytes and are processed as variable-length
/// integer in little-endian order. The function returns the type and the computed size.
///
/// # Parameters
/// * `stream`: The stream from which the bytes are read.
/// * `offset`: The offset of the stream.
///
/// # Returns
/// Returns an `io::Result` containing a tuple of the type and the computed size.
///
#[allow(unused)]
fn read_type_and_varint_size<R: Read>(stream: &mut R, offset: &mut usize) -> io::Result<(u8, u64)> {
    let (first_byte, continuation) = read_byte_and_check_continuation(stream)?;

    // Increment the offset by one byte
    *offset += 1;

    // Extract the type (bits 2, 3, 4 of the first byte)
    let type_bits = (first_byte & 0b0111_0000) >> 4;

    // Initialize size with the last 4 bits of the first byte
    let mut size: u64 = (first_byte & 0b0000_1111) as u64;
    let mut shift = 4; // Next byte will shift by 4 bits

    let mut more_bytes = continuation;
    while more_bytes {
        let (next_byte, continuation) = read_byte_and_check_continuation(stream)?;
        // Increment the offset by one byte
        *offset += 1;

        size |= (next_byte as u64) << shift;
        shift += 7; // Each subsequent byte contributes 7 more bits
        more_bytes = continuation;
    }

    Ok((type_bits, size))
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, env};
    use std::io::Cursor;

    use crate::internal::pack::Pack;
    use crate::internal::pack::read_byte_and_check_continuation;
    use crate::internal::pack::read_type_and_varint_size;

    #[test]
    fn test_pack_check_header() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git.pack");

        let mut f = std::fs::File::open(source).unwrap();
        let object_num = Pack::check_header(&mut f).unwrap();

        assert_eq!(object_num, 358109);
    }

    // #[test]
    // fn test_pack_decode() {
    //     let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
    //     source.push("tests/data/packs/git.pack");

    //     let mut f = std::fs::File::open(source).unwrap();
    //     let mut p = Pack { number: 0};
    //     p.decode(&mut f).unwrap();
    // }

    // Test case for a byte without a continuation bit (most significant bit is 0)
    #[test]
    fn test_read_byte_and_check_continuation_no_continuation() {
        let data = [0b0101_0101]; // 85 in binary, highest bit is 0
        let mut cursor = Cursor::new(data);
        let (value, more_bytes) = read_byte_and_check_continuation(&mut cursor).unwrap();

        assert_eq!(value, 85); // Expected value is 85
        assert!(!more_bytes); // No more bytes are expected
    }

    // Test case for a byte with a continuation bit (most significant bit is 1)
    #[test]
    fn test_read_byte_and_check_continuation_with_continuation() {
        let data = [0b1010_1010]; // 170 in binary, highest bit is 1
        let mut cursor = Cursor::new(data);
        let (value, more_bytes) = read_byte_and_check_continuation(&mut cursor).unwrap();

        assert_eq!(value, 42); // Expected value is 42 (170 - 128)
        assert!(more_bytes); // More bytes are expected
    }

    // Test cases for edge values, like the minimum and maximum byte values
    #[test]
    fn test_read_byte_and_check_continuation_edge_cases() {
        // Test the minimum value (0)
        let data = [0b0000_0000];
        let mut cursor = Cursor::new(data);
        let (value, more_bytes) = read_byte_and_check_continuation(&mut cursor).unwrap();

        assert_eq!(value, 0); // Expected value is 0
        assert!(!more_bytes); // No more bytes are expected

        // Test the maximum value (255)
        let data = [0b1111_1111];
        let mut cursor = Cursor::new(data);
        let (value, more_bytes) = read_byte_and_check_continuation(&mut cursor).unwrap();

        assert_eq!(value, 127); // Expected value is 127 (255 - 128)
        assert!(more_bytes); // More bytes are expected
    }

    // Test with a single byte where msb is 0 (no continuation)
    #[test]
    fn test_single_byte_no_continuation() {
        let data = [0b0101_0101]; // Type: 5 (101), Size: 5 (0101)
        let mut offset: usize = 0;
        let mut cursor = Cursor::new(data);
        let (type_bits, size) = read_type_and_varint_size(&mut cursor, &mut offset).unwrap();

        assert_eq!(offset, 1); // Offset is 1
        assert_eq!(type_bits, 5); // Expected type is 2
        assert_eq!(size, 5); // Expected size is 5
    }

    // Test with multiple bytes, where continuation occurs
    #[test]
    fn test_multiple_bytes_with_continuation() {
        // Type: 5 (101), Sizes: 5 (0101), 3 (0000011) in little-endian order
        let data = [0b1101_0101, 0b0000_0011]; // Second byte's msb is 0
        let mut offset: usize = 0;
        let mut cursor = Cursor::new(data);
        let (type_bits, size) = read_type_and_varint_size(&mut cursor, &mut offset).unwrap();

        assert_eq!(offset, 2); // Offset is 2
        assert_eq!(type_bits, 5); // Expected type is 5
        // Expected size 000000110101
        // 110101  = 1 * 2^5 + 1 * 2^4 + 0 * 2^3 + 1 * 2^2 + 0 * 2^1 + 1 * 2^0= 53
        assert_eq!(size, 53);
    }

    // Test with edge case where size is spread across multiple bytes
    #[test]
    fn test_edge_case_size_spread_across_bytes() {
        // Type: 1 (001), Sizes: 15 (1111) in little-endian order
        let data = [0b0001_1111, 0b0000_0010]; // Second byte's msb is 1 (continuation)
        let mut offset: usize = 0;
        let mut cursor = Cursor::new(data);
        let (type_bits, size) = read_type_and_varint_size(&mut cursor, &mut offset).unwrap();

        assert_eq!(offset, 1); // Offset is 1
        assert_eq!(type_bits, 1); // Expected type is 1
        // Expected size is 15 
        assert_eq!(size, 15);
    }
}