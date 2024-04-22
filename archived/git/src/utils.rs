//!
//!
//!
//!

use std::{
    io::{self, Read, Seek, SeekFrom},
    path::PathBuf,
    vec,
};

use crate::hash::Hash;

const TYPE_BITS: u8 = 3;
const VAR_INT_ENCODING_BITS: u8 = 7;
const TYPE_BYTE_SIZE_BITS: u8 = VAR_INT_ENCODING_BITS - TYPE_BITS;
const VAR_INT_CONTINUE_FLAG: u8 = 1 << VAR_INT_ENCODING_BITS;

/// Preserve the last bits of value binary
///
fn keep_bits(value: usize, bits: u8) -> usize {
    value & ((1 << bits) - 1)
}

/// Read the next N bytes from the reader
///
#[inline]
pub fn read_bytes<R: Read, const N: usize>(stream: &mut R) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    stream.read_exact(&mut bytes)?;

    Ok(bytes)
}

#[inline]
pub fn get_7bit_count(mut n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut count = 0;
    while n > 0 {
        count += 1;
        n >>= 7;
    }
    count
}

/// Read a u32 from the reader
///
pub fn read_u32<R: Read>(stream: &mut R) -> io::Result<u32> {
    let bytes = read_bytes(stream)?;

    Ok(u32::from_be_bytes(bytes))
}

pub fn read_chars<R: Read>(input: &mut R, buf: &mut Vec<u8>, size: usize) -> io::Result<()> {
    buf.resize(size, 0); // Resize the buffer to the specified size
    let mut bytes_read = 0;
    while bytes_read < size {
        match input.read(&mut buf[bytes_read..size]) {
            Ok(0) => break, // Reached EOF
            Ok(n) => bytes_read += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    buf.resize(bytes_read, 0); // Resize the buffer to the actual number of bytes read
    Ok(())
}
/// Read a hash from the reader
///
///
#[inline]
pub fn read_hash<R: Read>(stream: &mut R) -> io::Result<Hash> {
    let bytes = read_bytes(stream)?;

    Ok(Hash(bytes))
}

/// Read a vec until the delimiter is read
///
pub fn read_until_delimiter<R: Read>(stream: &mut R, delimiter: u8) -> io::Result<Vec<u8>> {
    let mut bytes = vec![];

    loop {
        let [byte] = read_bytes(stream)?;
        if byte == delimiter {
            break;
        }

        bytes.push(byte);
    }

    Ok(bytes)
}

/// Returns whether the first bit of u8 is 1 and returns the 7-bit truth value
///
pub fn read_var_int_byte<R: Read>(stream: &mut R) -> io::Result<(u8, bool)> {
    let [byte] = read_bytes(stream)?;
    let value = byte & !VAR_INT_CONTINUE_FLAG;
    let more_bytes = byte & VAR_INT_CONTINUE_FLAG != 0;

    Ok((value, more_bytes))
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

///
///
pub fn write_size_encoding(number: usize) -> Vec<u8> {
    let mut num = vec![];
    let mut number = number;

    loop {
        if number >> VAR_INT_ENCODING_BITS > 0 {
            num.push((number & 0x7f) as u8 | 0x80);
        } else {
            num.push((number & 0x7f) as u8);
            break;
        }

        number >>= VAR_INT_ENCODING_BITS;
    }

    num
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

/// The offset for an OffsetDelta object
/// # Arguments
///
/// * `stream`: Input Data Stream to read
/// * `consume`: This variable records the number of bytes consumed.
///
pub fn read_offset_encoding<R: Read>(stream: &mut R, consume: &mut usize) -> io::Result<u64> {
    // Like the object length, the offset for an OffsetDelta object
    // is stored in a variable number of bytes,
    // with the most significant bit of each byte indicating whether more bytes follow.
    // However, the object length encoding allows redundant values,
    // e.g. the 7-bit value [n] is the same as the 14- or 21-bit values [n, 0] or [n, 0, 0].
    // Instead, the offset encoding adds 1 to the value of each byte except the least significant one.
    // And just for kicks, the bytes are ordered from *most* to *least* significant.
    let mut value = 0;
    loop {
        let (byte_value, more_bytes) = read_var_int_byte(stream)?;
        *consume += 1;
        value = (value << VAR_INT_ENCODING_BITS) | byte_value as u64;
        if !more_bytes {
            return Ok(value);
        }

        value += 1;
    }
}

/// Converts a number to variable-length offset encoding.
///
/// The number is converted to a byte array using a variable-length encoding scheme where each byte
/// represents 7 bits of the number. The most significant bit of each byte is used as a continuation
/// marker, except for the last byte.
///
/// # Arguments
///
/// * `number` - The number to be encoded.
///
/// # Returns
///
/// A byte vector representing the variable-length offset encoding of the number.
pub fn write_offset_encoding(number: u64) -> Vec<u8> {
    let mut num = vec![];
    let mut number = number;

    // Encode the least significant 7 bits of the number in the first byte
    num.push((number & 0x7f) as u8);
    number >>= 7;

    // Encode the remaining bits in subsequent bytes
    while number > 0 {
        // Set the most significant bit to indicate continuation
        num.push(((number & 0x7f) - 1) as u8 | 0x80);
        number >>= 7;
    }

    // Reverse the byte vector to maintain the correct order
    num.reverse();

    num
}

/// Reads a partial integer from a stream.
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

/// Seeks to the specified offset in the given file.
///
/// # Arguments
///
/// * `file` - A mutable reference to the file to seek.
/// * `offset` - The offset to seek to, measured in bytes from the start of the file.
///
/// # Returns
///
/// This function returns an `io::Result<()>` indicating the success or failure of the seek operation.
/// If the seek operation is successful, `Ok(())` is returned. Otherwise, an `Err` variant is returned
/// with an `io::Error` describing the specific error that occurred.
///
pub fn seek<R: Seek>(file: &mut R, offset: u64) -> io::Result<()> {
    file.seek(SeekFrom::Start(offset))?;

    Ok(())
}

/// Retrieves the current offset position within a seekable file.
///
/// # Arguments
///
/// * `file` - A mutable reference to a seekable file implementing the `Seek` trait.
///
/// # Returns
///
/// This function returns an `io::Result<u64>` indicating the current offset position within the file.
/// If the operation is successful, the current offset position is returned as `Ok(offset)`.
/// Otherwise, an `Err` variant is returned with an `io::Error` describing the specific error that occurred.
///
pub fn get_offset<R: Seek>(file: &mut R) -> io::Result<u64> {
    file.stream_position()
}

/// Reads and decompresses a zlib stream from a file, applying a custom reader function to the decompressed data.
/// The function seeks to the end of the zlib stream after reading.
///
/// # Arguments
///
/// * `file` - A mutable reference to a file to read the zlib stream from.
/// * `reader` - A closure that takes a mutable reference to a `ZlibDecoder` and returns a result of type `T`.
///
/// # Returns
///
/// This function returns a result of type `Result<T, GitError>`. If the operation is successful, the result
/// of the reader function is returned as `Ok(result)`. Otherwise, an `Err` variant is returned, wrapping
/// a `GitError` that describes the specific error that occurred.
///
// pub fn read_zlib_stream_exact<T, F>(file: &mut (impl Read+Seek), reader: F) -> Result<T, GitError>
// where
//     F: FnMut(&mut ZlibDecoder<impl Read+Seek>) -> Result<T, GitError>,
// {
//     // Get the current offset position within the file
//     let offset = get_offset(file).unwrap();

//     // Create a zlib decoder for the file
//     let mut decompressed = ZlibDecoder::new(file);

//     // Invoke the reader function, passing the zlib decoder
//     let result = reader(&mut decompressed);

//     // Calculate the end position of the zlib stream
//     let zlib_end = offset + decompressed.total_in();

//     // Seek to the end of the zlib stream
//     seek(decompressed.into_inner(), zlib_end).unwrap();

//     result
// }

///
///
///
pub fn get_pack_raw_data(data: Vec<u8>) -> Vec<u8> {
    let result = &data[12..data.len() - 20];
    result.to_vec()
}

///
///
///
fn get_hash_form_filename(filename: &str) -> String {
    String::from(&filename[5..45])
}

/// Return a list of pack files in the pack directory.
pub fn find_all_pack_file(pack_dir: &str) -> (Vec<PathBuf>, Vec<Hash>) {
    let mut file_path = vec![];
    let mut hash_list = vec![];
    let mut object_root = std::path::PathBuf::from(pack_dir);

    let paths = std::fs::read_dir(&object_root).unwrap();

    for path in paths.flatten() {
        let file_name = path.file_name();
        let file_name = file_name.to_str().unwrap();

        // only find the .pack file, and find the .idx file
        if &file_name[file_name.len() - 4..] == "pack" {
            let hash_string = get_hash_form_filename(file_name);
            let hash = Hash::new_from_str(&hash_string);
            hash_list.push(hash);

            object_root.push(file_name);
            file_path.push(object_root.clone());
            object_root.pop();
        }
    }

    (file_path, hash_list)
}
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;
pub fn compress_zlib(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed_data = encoder.finish()?;
    Ok(compressed_data)
}
/// GET the env value from var str
pub fn get_env_number<T:std::str::FromStr>(var_name:&str, value :&mut T) {
    if let Ok(env_value)  = std::env::var(var_name){
        if let Ok(true_value) = env_value.parse::<T>(){
            *value = true_value;
        }
    }
}
#[cfg(test)]
mod test {
    use crate::utils::*;
    use std::env;

    #[test]
    fn test_get_env_value(){
        env::set_var("GIT_INTERNAL_DECODE_STORAGE_TQUEUE_SIZE", "10");
        env::set_var("GIT_INTERNAL_DECODE_CACHE_SIZE", "1000");
        env::set_var("GIT_INTERNAL_DECODE_STORAGE_BATCH_SIZE", "10000");
        
        let mut save_task_wait_number=0; // the most await save thread amount
        get_env_number("GIT_INTERNAL_DECODE_STORAGE_TQUEUE_SIZE", &mut save_task_wait_number);
        assert_eq!(save_task_wait_number, 10);

        let mut cache_size =0; // the most await save thread amount
        get_env_number("GIT_INTERNAL_DECODE_CACHE_SIZE", &mut cache_size);
        assert_eq!(cache_size, 1000);

        let mut batch_size =0; // the most await save thread amount
        get_env_number("GIT_INTERNAL_DECODE_STORAGE_BATCH_SIZE", &mut batch_size);
        assert_eq!(batch_size, 10000);
    }
}
