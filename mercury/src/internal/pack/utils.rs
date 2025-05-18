use sha1::{Digest, Sha1};
use std::fs;
use std::io::{self, Read};
use std::path::Path;

use crate::hash::SHA1;
use crate::internal::object::types::ObjectType;

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
pub fn is_eof(reader: &mut dyn Read) -> bool {
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
pub fn read_byte_and_check_continuation<R: Read>(stream: &mut R) -> io::Result<(u8, bool)> {
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
pub fn read_type_and_varint_size<R: Read>(
    stream: &mut R,
    offset: &mut usize,
) -> io::Result<(u8, usize)> {
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
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "VarInt too long",
            ));
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

/// The offset for an OffsetDelta object(big-endian order)
/// # Arguments
///
/// * `stream`: Input Data Stream to read
/// # Returns
/// * (`delta_offset`(unsigned), `consume`)
pub fn read_offset_encoding<R: Read>(stream: &mut R) -> io::Result<(u64, usize)> {
    // Like the object length, the offset for an OffsetDelta object
    // is stored in a variable number of bytes,
    // with the most significant bit of each byte indicating whether more bytes follow.
    // However, the object length encoding allows redundant values,
    // e.g. the 7-bit value [n] is the same as the 14- or 21-bit values [n, 0] or [n, 0, 0].
    // Instead, the offset encoding adds 1 to the value of each byte except the least significant one.
    // And just for kicks, the bytes are ordered from *most* to *least* significant.
    let mut value = 0;
    let mut offset = 0;
    loop {
        let (byte_value, more_bytes) = read_byte_and_check_continuation(stream)?;
        offset += 1;
        value = (value << 7) | byte_value as u64;
        if !more_bytes {
            return Ok((value, offset));
        }

        value += 1; //important!: for n >= 2 adding 2^7 + 2^14 + ... + 2^(7*(n-1)) to the result
    }
}

/// Read the next N bytes from the reader
///
#[inline]
pub fn read_bytes<R: Read, const N: usize>(stream: &mut R) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    stream.read_exact(&mut bytes)?;

    Ok(bytes)
}

/// Reads a partial integer from a stream. (little-endian order)
///
/// # Arguments
///
/// * `stream` - A mutable reference to a readable stream.
/// * `bytes` - The number of bytes to read from the stream.
/// * `present_bytes` - A mutable reference to a byte indicating which bits are present in the integer value.
///
/// # Returns
///
/// This function returns a result of type `io::Result<usize>`. If the operation is successful, the integer value
/// read from the stream is returned as `Ok(value)`. Otherwise, an `Err` variant is returned, wrapping an `io::Error`
/// that describes the specific error that occurred.
pub fn read_partial_int<R: Read>(
    stream: &mut R,
    bytes: u8,
    present_bytes: &mut u8,
) -> io::Result<usize> {
    let mut value: usize = 0;

    // Iterate over the byte indices
    for byte_index in 0..bytes {
        // Check if the current bit is present
        if *present_bytes & 1 != 0 {
            // Read a byte from the stream
            let [byte] = read_bytes(stream)?;

            // Add the byte value to the integer value
            value |= (byte as usize) << (byte_index * 8);
        }

        // Shift the present bytes to the right
        *present_bytes >>= 1;
    }

    Ok(value)
}

/// Reads the base size and result size of a delta object from the given stream.
///
/// **Note**: The stream MUST be positioned at the start of the delta object.
///
/// The base size and result size are encoded as variable-length integers in little-endian order.
///
/// The base size is the size of the base object, and the result size is the size of the result object.
///
/// # Parameters
/// * `stream`: The stream from which the sizes are read.
///
/// # Returns
/// Returns a tuple containing the base size and result size.
///
pub fn read_delta_object_size<R: Read>(stream: &mut R) -> io::Result<(usize, usize)> {
    let base_size = read_varint_le(stream)?.0 as usize;
    let result_size = read_varint_le(stream)?.0 as usize;
    Ok((base_size, result_size))
}

/// Calculate the SHA1 hash of the given object.
/// <br> "`<type> <size>\0<content>`"
/// <br> data: The decompressed content of the object
pub fn calculate_object_hash(obj_type: ObjectType, data: &Vec<u8>) -> SHA1 {
    let mut hash = Sha1::new();
    // Header: "<type> <size>\0"
    hash.update(obj_type.to_bytes());
    hash.update(b" ");
    hash.update(data.len().to_string());
    hash.update(b"\0");

    // Decompressed data(raw content)
    hash.update(data);

    let re: [u8; 20] = hash.finalize().into();
    SHA1(re)
}
/// Create an empty directory or clear the existing directory.
pub fn create_empty_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let dir = path.as_ref();
    // 删除整个文件夹
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    // 重新创建文件夹
    fs::create_dir_all(dir)?;
    Ok(())
}

/// Count the number of files in a directory and its subdirectories.
pub fn count_dir_files(path: &Path) -> io::Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            count += count_dir_files(&path)?;
        } else {
            count += 1;
        }
    }
    Ok(count)
}

/// Count the time taken to execute a block of code.
#[macro_export]
macro_rules! time_it {
    ($msg:expr, $block:block) => {{
        let start = std::time::Instant::now();
        let result = $block;
        let elapsed = start.elapsed();
        // println!("{}: {:?}", $msg, elapsed);
        tracing::info!("{}: {:?}", $msg, elapsed);
        result
    }};
}

#[cfg(test)]
mod tests {
    use crate::internal::object::types::ObjectType;
    use std::io;
    use std::io::Cursor;
    use std::io::Read;

    use crate::internal::pack::utils::*;

    #[test]
    fn test_calc_obj_hash() {
        let hash = calculate_object_hash(ObjectType::Blob, &b"a".to_vec());
        assert_eq!(hash.to_string(), "2e65efe2a145dda7ee51d1741299f848e5bf752e");
    }

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
                Err(io::Error::other("error"))
            }
        }

        let mut reader = BrokenReader;
        assert!(!is_eof(&mut reader));
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
        let data = vec![
            0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01,
        ];
        let mut cursor = Cursor::new(data);
        let result = read_varint_le(&mut cursor);

        assert!(result.is_err());
    }

    #[test]
    fn test_read_offset_encoding() {
        let data: Vec<u8> = vec![0b_1101_0101, 0b_0000_0101];
        let mut cursor = Cursor::new(data);
        let result = read_offset_encoding(&mut cursor);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (11013, 2));
    }
}
