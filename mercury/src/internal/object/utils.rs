use std::io::{self, Read, Write};

use flate2::{Compression, write::ZlibEncoder};

const TYPE_BITS: u8 = 3;
const VAR_INT_ENCODING_BITS: u8 = 7;
const TYPE_BYTE_SIZE_BITS: u8 = VAR_INT_ENCODING_BITS - TYPE_BITS;
const VAR_INT_CONTINUE_FLAG: u8 = 1 << VAR_INT_ENCODING_BITS;

/// Parses a byte slice into a `usize` representing the size of a Git object.
///
/// This function is intended to be used for converting the bytes, which represent the size portion
/// in a Git object, back into a `usize`. This size is typically compared with the actual length of
/// the object's data part to ensure data integrity.
///
/// # Parameters
/// * `bytes`: A byte slice (`&[u8]`) representing the size in a serialized Git object.
///
/// # Returns
/// Returns a `Result` which is:
/// * `Ok(usize)`: On successful parsing, returns the size as a `usize`.
/// * `Err(Box<dyn std::error::Error>)`: On failure, returns an error in a Box. This error could be
///   due to invalid UTF-8 encoding in the byte slice or a failure to parse the byte slice as a `usize`.
///
/// # Errors
/// This function handles two main types of errors:
/// 1. `Utf8Error`: If the byte slice is not a valid UTF-8 string, which is necessary for the size representation.
/// 2. `ParseIntError`: If the byte slice does not represent a valid `usize` value.
pub fn parse_size_from_bytes(bytes: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
    let size_str = std::str::from_utf8(bytes)?;
    Ok(size_str.parse::<usize>()?)
}

/// Preserve the last bits of value binary
///
fn keep_bits(value: usize, bits: u8) -> usize {
    value & ((1 << bits) - 1)
}
/// Read the first few fields of the object and parse
///
pub fn read_type_and_size<R: Read>(stream: &mut R) -> io::Result<(u8, usize)> {
    // Object type and uncompressed pack data size
    // are stored in a "size-encoding" variable-length integer.
    // Bits 4 through 6 store the type and the remaining bits store the size.
    let value = read_size_encoding(stream)?;
    let object_type = keep_bits(value >> TYPE_BYTE_SIZE_BITS, TYPE_BITS) as u8;
    let size = keep_bits(value, TYPE_BYTE_SIZE_BITS)
        | (value >> VAR_INT_ENCODING_BITS << TYPE_BYTE_SIZE_BITS);

    Ok((object_type, size))
}

/// Read the type and size of the object
///
pub fn read_size_encoding<R: Read>(stream: &mut R) -> io::Result<usize> {
    let mut value = 0;
    let mut length = 0;

    loop {
        let (byte_value, more_bytes) = read_var_int_byte(stream).unwrap();
        value |= (byte_value as usize) << length;
        if !more_bytes {
            return Ok(value);
        }

        length += VAR_INT_ENCODING_BITS;
    }
}

/// Returns whether the first bit of u8 is 1 and returns the 7-bit truth value
///
pub fn read_var_int_byte<R: Read>(stream: &mut R) -> io::Result<(u8, bool)> {
    let [byte] = read_bytes(stream)?;
    let value = byte & !VAR_INT_CONTINUE_FLAG;
    let more_bytes = byte & VAR_INT_CONTINUE_FLAG != 0;

    Ok((value, more_bytes))
}

/// Read the next N bytes from the reader
///
#[inline]
pub fn read_bytes<R: Read, const N: usize>(stream: &mut R) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    stream.read_exact(&mut bytes)?;

    Ok(bytes)
}

pub fn compress_zlib(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed_data = encoder.finish()?;
    Ok(compressed_data)
}

#[cfg(test)]
mod tests {
    use crate::internal::object::utils::parse_size_from_bytes;

    #[test]
    fn test_parse_size_from_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let size: usize = 12345;
        let size_bytes = size.to_string().as_bytes().to_vec();

        let parsed_size = parse_size_from_bytes(&size_bytes)?;

        assert_eq!(size, parsed_size);
        Ok(())
    }
}
