//! 
//! ## Reference
//! 1. Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
//!

use std::io::{self, Read, BufRead, Seek};

use sha1::{Sha1, Digest};
use flate2::bufread::ZlibDecoder;

use crate::errors::GitError;
use crate::hash::SHA1;



///
/// 
/// 
#[allow(unused)]
pub struct Pack {
    pub number: usize,
}

/// `HashCounter` is a wrapper around a reader that also computes the SHA1 hash of the data read.
///
/// It is designed to work with any reader that implements `BufRead`.
///
/// Fields:
/// * `inner`: The inner reader.
/// * `hash`: The SHA1 hash state.
/// * `count_hash`: A flag to indicate whether to compute the hash while reading.
pub struct HashCounter<R> {
    inner: R,
    hash: Sha1,
    count_hash: bool,
}

impl<R> HashCounter<R>
where
    R: BufRead,
{
    /// Constructs a new `HashCounter` with the given reader and a flag to enable or disable hashing.
    ///
    /// # Parameters
    /// * `inner`: The reader to wrap.
    /// * `count_hash`: If `true`, the hash is computed while reading; otherwise, it is not.
    pub fn new(inner: R, count_hash: bool) -> Self {
        Self {
            inner,
            hash: Sha1::new(), // Initialize a new SHA1 hasher
            count_hash,
        }
    }

    /// Returns the final SHA1 hash of the data read so far.
    ///
    /// This is a clone of the internal hash state finalized into a SHA1 hash.
    pub fn final_hash(&self) -> SHA1 {
        let re: [u8; 20] = self.hash.clone().finalize().into(); // Clone, finalize, and convert the hash into bytes
        SHA1(re)
    }
}

impl<R> BufRead for HashCounter<R>
where
    R: BufRead,
{
    /// Provides access to the internal buffer of the wrapped reader without consuming it.
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf() // Delegate to the inner reader
    }

    /// Consumes data from the buffer and updates the hash if `count_hash` is true.
    ///
    /// # Parameters
    /// * `amt`: The amount of data to consume from the buffer.
    fn consume(&mut self, amt: usize) {
        let buffer = self.inner.fill_buf().expect("Failed to fill buffer");
        if self.count_hash {
            self.hash.update(&buffer[..amt]); // Update hash with the data being consumed
        }
        self.inner.consume(amt); // Consume the data from the inner reader
    }
}

impl<R> Read for HashCounter<R>
where
    R: BufRead,
{
    /// Reads data into the provided buffer and updates the hash if `count_hash` is true.
    ///
    /// # Parameters
    /// * `buf`: The buffer to read data into.
    ///
    /// # Returns
    /// Returns the number of bytes read.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let o = self.inner.read(buf)?; // Read data into the buffer
        if self.count_hash {
            self.hash.update(&buf[..o]); // Update hash with the data being read
        }
        Ok(o) // Return the number of bytes read
    }
}

/// 
/// 
impl Pack {
    /// Checks and reads the header of a Git pack file.
    ///
    /// This function reads the first 12 bytes of a pack file, which include the "PACK" magic identifier,
    /// the version number, and the number of objects in the pack. It verifies that the magic identifier
    /// is correct and that the version number is 2 (which is the version currently supported by Git).
    /// It also collects these header bytes for later use, such as for hashing the entire pack file.
    ///
    /// # Parameters
    /// * `pack`: A mutable reference to an object implementing the `Read` trait,
    ///           representing the source of the pack file data (e.g., file, memory stream).
    ///
    /// # Returns
    /// A `Result` which is:
    /// * `Ok((u32, Vec<u8>))`: On successful reading and validation of the header, returns a tuple where:
    ///     - The first element is the number of objects in the pack file (`u32`).
    ///     - The second element is a vector containing the bytes of the pack file header (`Vec<u8>`).
    /// * `Err(GitError)`: On failure, returns a `GitError` with a description of the issue.
    ///
    /// # Errors
    /// This function can return an error in the following situations:
    /// * If the pack file does not start with the "PACK" magic identifier.
    /// * If the pack file's version number is not 2.
    /// * If there are any issues reading from the provided `pack` source.
    pub fn check_header(pack: &mut (impl Read + BufRead)) -> Result<(u32, Vec<u8>), GitError> {
        // A vector to store the header data for hashing later
        let mut header_data = Vec::new();
    
        // Read the first 4 bytes which should be "PACK"
        let mut magic = [0; 4];
        // Read the magic "PACK" identifier
        let result = pack.read_exact(&mut magic); 
        match result {
            Ok(_) => {
                // Store these bytes for later
                header_data.extend_from_slice(&magic);

                // Check if the magic bytes match "PACK"
                if magic != *b"PACK" {
                    // If not, return an error indicating invalid pack header
                    return Err(GitError::InvalidPackHeader(format!(
                        "{},{},{},{}",
                        magic[0], magic[1], magic[2], magic[3]
                    )));
                }
            },
            Err(_e) => {
                // If there is an error in reading, return a GitError
                return Err(GitError::InvalidPackHeader(format!(
                    "{},{},{},{}",
                    magic[0], magic[1], magic[2], magic[3]
                )));
            }
        }
        
        // Read the next 4 bytes for the version number
        let mut version_bytes = [0; 4];
        let result = pack.read_exact(&mut version_bytes); // Read the version number
        match result {
            Ok(_) => {
                // Store these bytes
                header_data.extend_from_slice(&version_bytes);

                // Convert the version bytes to a u32 integer
                let version = u32::from_be_bytes(version_bytes);
                if version != 2 {
                    // Git currently supports version 2, so error if not version 2
                    return Err(GitError::InvalidPackFile(format!(
                        "Version Number is {}, not 2",
                        version
                    )));
                }
                // If read is successful, proceed
            }, 
            Err(_e) => {
                // If there is an error in reading, return a GitError
                return Err(GitError::InvalidPackHeader(format!(
                    "{},{},{},{}",
                    version_bytes[0], version_bytes[1], version_bytes[2], version_bytes[3]
                )));
            }
        }
    
        // Read the next 4 bytes for the number of objects in the pack
        let mut object_num_bytes = [0; 4];
        // Read the number of objects
        let result = pack.read_exact(&mut object_num_bytes);
        match result {
            Ok(_) => {
                // Store these bytes
                header_data.extend_from_slice(&object_num_bytes);
                // Convert the object number bytes to a u32 integer
                let object_num = u32::from_be_bytes(object_num_bytes);
                // Return the number of objects and the header data for further processing
                Ok((object_num, header_data))
            }, 
            Err(_e) => {
                // If there is an error in reading, return a GitError
                Err(GitError::InvalidPackHeader(format!(
                    "{},{},{},{}",
                    object_num_bytes[0], object_num_bytes[1], object_num_bytes[2], object_num_bytes[3]
                )))
            }
            
        }
    }

    /// Decompresses data from a given Read and BufRead source using Zlib decompression.
    ///
    /// # Parameters
    /// * `pack`: A source that implements both Read and BufRead traits (e.g., file, network stream).
    /// * `expected_size`: The expected decompressed size of the data.
    ///
    /// # Returns
    /// Returns a `Result` containing either:
    /// * A tuple with a `Vec<u8>` of decompressed data, a `Vec<u8>` of the original compressed data,
    ///   and the total number of input bytes processed,
    /// * Or a `GitError` in case of a mismatch in expected size or any other reading error.
    ///
    pub fn decompress_data(&mut self, pack: &mut (impl Read + BufRead + Send), expected_size: usize) -> Result<(Vec<u8>, usize), GitError> {        
        // Create a buffer with the expected size for the decompressed data
        let mut buf = Vec::with_capacity(expected_size);
        // Create a new Zlib decoder with the original data
        let mut deflate = ZlibDecoder::new(pack);

        // Attempt to read data to the end of the buffer
        match deflate.read_to_end(&mut buf) {
            Ok(_) => {
                // Check if the length of the buffer matches the expected size
                if buf.len() != expected_size {
                    Err(GitError::InvalidPackFile(format!(
                        "The object size {} does not match the expected size {}",
                        buf.len(),
                        expected_size
                    )))
                } else {
                    // If everything is as expected, return the buffer, the original data, and the total number of input bytes processed
                    Ok((buf, deflate.total_in() as usize))
                }
            },
            Err(e) => {
                // If there is an error in reading, return a GitError
                Err(GitError::InvalidPackFile(format!("Decompression error: {}", e)))
            }
        }
    }

    /// Decodes a pack object from a given Read and BufRead source and returns the original compressed data.
    ///
    /// # Parameters
    /// * `pack`: A source that implements both Read and BufRead traits.
    /// * `offset`: A mutable reference to the current offset within the pack.
    ///
    /// # Returns
    /// Returns a `Result` containing either:
    /// * A tuple of the next offset in the pack and the original compressed data as `Vec<u8>`,
    /// * Or a `GitError` in case of any reading or decompression error.
    ///
    pub fn decode_pack_object(&mut self, pack: &mut (impl Read + BufRead + Send), offset: &mut usize) -> Result<usize, GitError> {
        // Attempt to read the type and size, handle potential errors
        let (type_bits, size) = match read_type_and_varint_size(pack, offset) {
            Ok(result) => result,
            Err(e) => {
                // Handle the error e.g., by logging it or converting it to GitError
                // and then return from the function
                return Err(GitError::InvalidPackFile(format!("Read error: {}", e)));
            }
        };

        match type_bits {
            1..=4 => {
                let (_, object_offset) = self.decompress_data(pack, size)?;
                *offset += object_offset;
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
                
                let (_, object_offset) = self.decompress_data(pack, size)?;
                *offset += object_offset;
            },
            7 => {
                // Read 20 bytes to get the reference object SHA1 hash
                let mut buf_ref = [0; 20];
                pack.read_exact(&mut buf_ref).unwrap();
                let _ref_sha1 = SHA1::from_bytes(buf_ref.as_ref());
                // Offset is incremented by 20 bytes
                *offset += 20;

                let (_, object_offset) = self.decompress_data(pack, size)?;
                *offset += object_offset;
            },
            _ => {
                return Err(GitError::InvalidPackFile(format!(
                    "Unknown object type number: {}",
                    type_bits
                )));
            }
        }

        Ok(*offset)
    }


    ///
    /// 
    /// 
    pub fn decode(&mut self, pack: &mut (impl Read + BufRead + Seek + Send)) -> Result<(), GitError> {
        let count_hash: bool = true;
        let mut render = HashCounter::new(io::BufReader::new(pack), count_hash);


        let result = Pack::check_header(&mut render);
        match result {
            Ok((object_num, _)) => {
                self.number = object_num as usize;
            },
            Err(e) => {
                return Err(e);
            }
        }
        
        let mut offset: usize = 12;
        let mut i = 1;

        while i <= self.number {
            let result: Result<usize, GitError> = self.decode_pack_object(&mut render, &mut offset);
            match result {
                Ok(_) => {},
                Err(e) => {
                    return Err(e);
                }
            }

            i += 1;
        }

        let render_hash = render.final_hash();
        let mut tailer_buf= [0; 20];
        render.read_exact(&mut tailer_buf).unwrap();
        let tailer_signature = SHA1::from_bytes(tailer_buf.as_ref());

        if render_hash != tailer_signature {
            return Err(GitError::InvalidPackFile(format!(
                "The pack file hash {} does not match the tailer hash {}",
                render_hash.to_plain_str(),
                tailer_signature.to_plain_str()
            )));
        }

        let end = is_eof(&mut render);
        if !end {
            return Err(GitError::InvalidPackFile(
                "The pack file is not at the end".to_string()
            ));
        }

        Ok(())
    }

}

/// Checks if the reader has reached EOF (end of file).
/// 
/// It attempts to read a single byte from the reader into a buffer.
/// If `Ok(0)` is returned, it means no byte was read, indicating 
/// that the end of the stream has been reached and there is no more
/// data left to read.
///
/// Any other return value means that data was successfully read, so
/// the reader has not reached the end yet.  
///
/// # Arguments
/// 
/// * `reader` - The reader to check for EOF state  
///   It must implement the `std::io::Read` trait
///
/// # Returns  
/// 
/// true if the reader reached EOF, false otherwise
#[allow(unused)]
fn is_eof(reader: &mut dyn Read) -> bool {
    let mut buf = [0; 1];
    matches!(reader.read(&mut buf), Ok(0))
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
#[allow(unused)]
pub fn read_varint_le<R: Read>(reader: &mut R) -> io::Result<(u64, usize)> {
    // The decoded value
    let mut value: u64 = 0;
    // Bit shift for the next byte
    let mut shift = 0;
    // Number of bytes read
    let mut offset = 0; 

    loop {
        // A buffer to read a single byte
        let mut buf = [0; 1];
        // Read one byte from the reader
        reader.read_exact(&mut buf)?;

        // The byte just read
        let byte = buf[0]; 
        if shift > 63 { 
            // VarInt too long for u64
            return Err(io::Error::new(io::ErrorKind::InvalidData, "VarInt too long"));
        }

        // Take the lower 7 bits of the byte
        let byte_value = (byte & 0x7F) as u64; 
        // Add the byte value to the result, considering the shift
        value |= byte_value << shift; 

        // Increment the byte count
        offset += 1; 
        // Check if the MSB is 0 (last byte)
        if byte & 0x80 == 0 {
            break;
        }

        // Increment the shift for the next byte
        shift += 7;
    }

    Ok((value, offset))
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, env};
    use std::io;
    use std::io::Cursor;
    use std::io::BufReader;
    use std::io::prelude::*;
    

    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    
    use crate::internal::pack::Pack;
    use crate::internal::pack::read_byte_and_check_continuation;
    use crate::internal::pack::read_type_and_varint_size;
    use crate::internal::pack::read_varint_le;
    use crate::internal::pack::is_eof;

    #[test]
    fn eof() {
        let mut reader = Cursor::new(&b""[..]);
        assert!(is_eof(&mut reader));
    }

    #[test] 
    fn not_eof() {
        let mut reader = Cursor::new(&b"abc"[..]);
        assert!(!is_eof(&mut reader));
    }

    #[test]
    fn eof_midway() {
        let mut reader = Cursor::new(&b"abc"[..]);
        reader.read_exact(&mut [0; 2]).unwrap();
        assert!(!is_eof(&mut reader));
    }

    #[test]
    fn reader_error() {
        struct BrokenReader;
        impl Read for BrokenReader {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::Other, "error"))
            }
        }
        
        let mut reader = BrokenReader;
        assert!(!is_eof(&mut reader)); 
    }

    #[test]
    fn test_pack_check_header() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git-2d187177923cd618a75da6c6db45bb89d92bd504.pack");

        let f = std::fs::File::open(source).unwrap();
        let mut buf_reader = BufReader::new(f);
        let (object_num, _) = Pack::check_header(&mut buf_reader).unwrap();

        assert_eq!(object_num, 358109);
    }

    #[test]
    fn test_decompress_data() {
        let data = b"Hello, world!"; // Sample data to compress and then decompress
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        let compressed_data = encoder.finish().unwrap();

        // Create a cursor for the compressed data to simulate a Read + BufRead source
        let mut cursor: Cursor<Vec<u8>> = Cursor::new(compressed_data);
        let expected_size = data.len();

        // Decompress the data and assert correctness
        let mut p = Pack { number: 0};
        let result = p.decompress_data(&mut cursor, expected_size);
        match result {
            Ok((decompressed_data, _)) => {
                assert_eq!(decompressed_data, data);
            },
            Err(e) => panic!("Decompression failed: {:?}", e),
        }
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
        source.push("tests/data/packs/ref-delta-65d47638aa7cb7c39f1bd1d5011a415439b887a8.pack");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack { number: 0};
        p.decode(&mut buffered).unwrap();
    }

    #[test]
    fn test_pack_decode_with_large_file_with_delta_without_ref() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git-2d187177923cd618a75da6c6db45bb89d92bd504.pack");

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