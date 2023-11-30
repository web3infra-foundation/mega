//! 
//! ## Reference
//! 1. Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
//!

use std::io::{self, Read, BufRead};

use flate2::bufread::ZlibDecoder;

use common::utils;

use crate::errors::GitError;
use crate::hash::SHA1;

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
    pub fn decode_pack_object(&mut self, pack: &mut (impl Read + BufRead), offset: &mut usize) -> Result<usize, GitError> {
        let (type_bits, size) = read_type_and_varint_size(pack, offset).unwrap();

        match type_bits {
            1..=4 => {
                let mut buf = Vec::with_capacity(size);
                let mut deflate = ZlibDecoder::new(pack);
                deflate.read_to_end(&mut buf).unwrap();
                if buf.len() != size {
                    return Err(GitError::InvalidPackFile(format!(
                        "The object size {} does not match the expected size {}",
                        buf.len(),
                        size
                    )));
                } else {
                    *offset += deflate.total_in() as usize;
                }
            },
            5 => {
                return Err(GitError::InvalidPackFile(format!(
                    "The object type number {} is reserved for future use",
                    type_bits
                )));
            },
            6 => {
                let (_base_offset, step_offset) = read_varint_le(pack).unwrap();
                *offset += step_offset;
                
                let mut buf = Vec::with_capacity(size);
                let mut deflate = ZlibDecoder::new(pack);
                deflate.read_to_end(&mut buf).unwrap();
                if buf.len() != size {
                    return Err(GitError::InvalidPackFile(format!(
                        "The object size {} does not match the expected size {}",
                        buf.len(),
                        size
                    )));
                } else {
                    *offset += deflate.total_in() as usize;
                }
            },
            7 => {
                // Read 20 bytes to get the SHA1 hash
                let mut buf_ref = [0; 20];
                pack.read_exact(&mut buf_ref).unwrap();
                let sha1 = SHA1::from_bytes(buf_ref.as_ref());
                println!("sha1: {}", sha1.to_plain_str());
                *offset += 20;

                let mut buf = Vec::with_capacity(size);
                let mut deflate = ZlibDecoder::new(pack);
                deflate.read_to_end(&mut buf).unwrap();
                if buf.len() != size {
                    return Err(GitError::InvalidPackFile(format!(
                        "The object size {} does not match the expected size {}",
                        buf.len(),
                        size
                    )));
                } else {
                    *offset += deflate.total_in() as usize;
                }
            },
            _ => {
                return Err(GitError::InvalidPackFile(format!(
                    "Unknown object type number: {}",
                    type_bits
                )));
            }
        }

        Ok(0)
    }

    ///
    /// 
    /// 
    pub fn decode(&mut self, pack: &mut (impl Read + BufRead)) -> Result<(), GitError> {
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
fn read_type_and_varint_size<R: Read>(stream: &mut R, offset: &mut usize) -> io::Result<(u8, usize)> {
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

    Ok((type_bits, size as usize))
}

/// Reads a variable-length integer (VarInt) encoded in little-endian format from a source implementing the Read trait.
/// 
/// The VarInt encoding uses the most significant bit (MSB) of each byte as a continuation bit.
/// The continuation bit being 1 indicates that there are following bytes.
/// The actual integer value is encoded in the remaining 7 bits of each byte.
///
/// # Parameters
/// * `reader`: A source implementing the Read trait (e.g., file, network stream).
///
/// # Returns
/// Returns a `Result` containing either:
/// * A tuple of the decoded `u64` value and the number of bytes read (`offset`).
/// * An `io::Error` in case of any reading error or if the VarInt is too long.
///
pub fn read_varint_le<R: Read>(reader: &mut R) -> io::Result<(u64, usize)> {
    let mut value: u64 = 0;     // The decoded value
    let mut shift = 0;          // Bit shift for the next byte
    let mut offset = 0;         // Number of bytes read

    loop {
        let mut buf = [0; 1];       // A buffer to read a single byte
        reader.read_exact(&mut buf)?;  // Read one byte from the reader

        let byte = buf[0];            // The byte just read
        if shift > 63 {               // VarInt too long for u64
            return Err(io::Error::new(io::ErrorKind::InvalidData, "VarInt too long"));
        }

        let byte_value = (byte & 0x7F) as u64; // Take the lower 7 bits of the byte
        value |= byte_value << shift;         // Add the byte value to the result, considering the shift

        offset += 1;                    // Increment the byte count
        if byte & 0x80 == 0 {           // Check if the MSB is 0 (last byte)
            break;
        }

        shift += 7;                     // Increment the shift for the next byte
    }

    Ok((value, offset)) // Return the decoded value and number of bytes read
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, env};
    use std::io::Cursor;
    use std::io::BufReader;

    use crate::internal::pack::Pack;
    use crate::internal::pack::read_byte_and_check_continuation;
    use crate::internal::pack::read_type_and_varint_size;
    use crate::internal::pack::read_varint_le;

    #[test]
    fn test_pack_check_header() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git.pack");

        let mut f = std::fs::File::open(source).unwrap();
        let object_num = Pack::check_header(&mut f).unwrap();

        assert_eq!(object_num, 358109);
    }

    #[test]
    fn test_pack_decode_without_delta() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/pack-1d0e6c14760c956c173ede71cb28f33d921e232f.pack");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack { number: 0};
        p.decode(&mut buffered).unwrap();
    }

    #[test]
    fn test_pack_decode_with_ref_delta() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/ref-delta.pack");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack { number: 0};
        p.decode(&mut buffered).unwrap();
    }

    #[test]
    fn test_pack_decode_with_large_file_with_delta_without_ref() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git.pack");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack { number: 0};
        p.decode(&mut buffered).unwrap();
    }

    #[test]
    fn test_pack_decode_with_delta_without_ref() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/pack-d50df695086eea6253a237cb5ac44af1629e7ced.pack");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack { number: 0};
        p.decode(&mut buffered).unwrap();
    }

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

    #[test]
    fn test_read_varint_le_single_byte() {
        // Single byte: 0x05 (binary: 0000 0101)
        // Represents the value 5 with no continuation bit set.
        let data = vec![0x05];
        let mut cursor = Cursor::new(data);
        let (value, offset) = read_varint_le(&mut cursor).unwrap();

        assert_eq!(value, 5);
        assert_eq!(offset, 1);
    }

    #[test]
    fn test_read_varint_le_multiple_bytes() {
        // Two bytes: 0x85, 0x01 (binary: 1000 0101, 0000 0001)
        // Represents the value 133. First byte has the continuation bit set.
        let data = vec![0x85, 0x01];
        let mut cursor = Cursor::new(data);
        let (value, offset) = read_varint_le(&mut cursor).unwrap();

        assert_eq!(value, 133);
        assert_eq!(offset, 2);
    }

    #[test]
    fn test_read_varint_le_large_number() {
        // Five bytes: 0xFF, 0xFF, 0xFF, 0xFF, 0xF (binary: 1111 1111, 1111 1111, 1111 1111, 1111 1111, 0000 1111)
        // Represents the value 134,217,727. All continuation bits are set except in the last byte.
        let data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xF];
        let mut cursor = Cursor::new(data);
        let (value, offset) = read_varint_le(&mut cursor).unwrap();

        assert_eq!(value, 0xFFFFFFFF);
        assert_eq!(offset, 5);
    }

    #[test]
    fn test_read_varint_le_zero() {
        // Single byte: 0x00 (binary: 0000 0000)
        // Represents the value 0 with no continuation bit set.
        let data = vec![0x00];
        let mut cursor = Cursor::new(data);
        let (value, offset) = read_varint_le(&mut cursor).unwrap();

        assert_eq!(value, 0);
        assert_eq!(offset, 1);
    }

    #[test]
    fn test_read_varint_le_too_long() {
        let data = vec![0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01];
        let mut cursor = Cursor::new(data);
        let result = read_varint_le(&mut cursor);

        assert!(result.is_err());
    }
}