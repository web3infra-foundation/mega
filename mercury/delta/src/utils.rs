use std::io::Read;

const VAR_INT_ENCODING_BITS: u8 = 7;
const VAR_INT_CONTINUE_FLAG: u8 = 1 << VAR_INT_ENCODING_BITS;

/// Read the next N bytes from the reader
///
#[inline]
pub fn read_bytes<R: Read, const N: usize>(stream: &mut R) -> std::io::Result<[u8; N]> {
    let mut bytes = [0; N];
    stream.read_exact(&mut bytes)?;

    Ok(bytes)
}

// Read the type and size of the object
pub fn read_size_encoding<R: Read>(stream: &mut R) -> std::io::Result<usize> {
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
) -> std::io::Result<usize> {
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

/// Returns whether the first bit of u8 is 1 and returns the 7-bit truth value
///
pub fn read_var_int_byte<R: Read>(stream: &mut R) -> std::io::Result<(u8, bool)> {
    let [byte] = read_bytes(stream)?;
    let value = byte & !VAR_INT_CONTINUE_FLAG;
    let more_bytes = byte & VAR_INT_CONTINUE_FLAG != 0;

    Ok((value, more_bytes))
}
