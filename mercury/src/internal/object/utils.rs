//! 
//! 
//! 
//! 

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