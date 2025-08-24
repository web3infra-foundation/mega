//! In Git, the SHA-1 hash algorithm is widely used to generate unique identifiers for Git objects.
//! Each Git object corresponds to a unique SHA-1 hash value, which is used to identify the object's
//! location in the Git internal and mega database.
//!

use std::{fmt::Display, io};

use bincode::{Decode, Encode};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use sha1::Digest;

use crate::internal::object::types::ObjectType;

/// The [`SHA1`] struct, encapsulating a `[u8; 20]` array, is specifically designed to represent Git hash IDs.
/// In Git's context, these IDs are 40-character hexadecimal strings generated via the SHA-1 algorithm.
/// Each Git object receives a unique hash ID based on its content, serving as an identifier for its location
/// within the Git internal database. Utilizing a dedicated struct for these hash IDs enhances code readability and
/// maintainability by providing a clear, structured format for their manipulation and storage.
///
/// ### Change Log
///
/// In previous versions of the 'mega' project, `Hash` was used to denote hash values. However, in newer versions,
/// `SHA1` is employed for this purpose. Future updates plan to extend support to SHA256 and SHA512, or potentially
/// other hash algorithms. By abstracting the hash model to `Hash`, and using specific imports like `use crate::hash::SHA1`
/// or `use crate::hash::SHA256`, the codebase maintains a high level of clarity and maintainability. This design choice
/// allows for easier adaptation to different hash algorithms while keeping the underlying implementation consistent and
/// understandable. - Nov 26, 2023 (by @genedna)
///
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Deserialize,
    Serialize,
    Encode,
    Decode,
)]
pub struct SHA1(pub [u8; 20]);

/// Display trait for SHA1.
impl Display for SHA1 {
    /// Allows [`SHA1::to_string()`] to be used.
    /// Note: If you want a terminal-friendly colorized output, use [`SHA1::to_color_str()`].
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl AsRef<[u8]> for SHA1 {
    /// Returns a reference to the raw hash bytes.
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
/// Implementation of the [`std::str::FromStr`] trait for the [`SHA1`] type.
///
/// To effectively use the `from_str` method for converting a string to a `SHA1` object, consider the following:
///   1. The input string `s` should be a pre-calculated hexadecimal string, exactly 40 characters in length. This string
///      represents a SHA1 hash and should conform to the standard SHA1 hash format.
///   2. It is necessary to explicitly import the `FromStr` trait to utilize the `from_str` method. Include the import
///      statement `use std::str::FromStr;` in your code before invoking the `from_str` function. This import ensures
///      that the `from_str` method is available for converting strings to `SHA1` objects.
impl std::str::FromStr for SHA1 {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut h = SHA1::default();
        if s.len() != 40 {
            return Err("The length of the string is not 40".to_string());
        }
        let bytes = hex::decode(s).map_err(|e| e.to_string())?;
        h.0.copy_from_slice(bytes.as_slice());
        Ok(h)
    }
}

/// Implementation of the `SHA1` struct.
///
/// The naming conventions for the methods in this implementation are designed to be intuitive and self-explanatory:
///
/// 1. `new` Prefix:
///    Methods starting with `new` are used for computing an SHA-1 hash from given data, signifying the creation of
///    a new `SHA1` instance. For example, `pub fn new(data: &Vec<u8>) -> SHA1` takes a byte vector and calculates its SHA-1 hash.
///
/// 2. `from` Prefix:
///    Methods beginning with `from` are intended for creating a `SHA1` instance from an existing, pre-calculated value.
///    This implies direct derivation of the `SHA1` object from the provided input. For instance, `pub fn from_bytes(bytes: &[u8]) -> SHA1`
///    constructs a `SHA1` from a 20-byte array representing an SHA-1 hash.
///
/// 3. `to` Prefix:
///    Methods with the `to` prefix are used for outputting the `SHA1` value in various formats. This prefix indicates a transformation or
///    conversion of the `SHA1` instance into another representation. For example, `pub fn to_string(self) -> String` converts the SHA1
///    value to a plain hexadecimal string, and `pub fn to_data(self) -> Vec<u8>` converts it into a byte vector. The `to` prefix
///    thus serves as a clear indicator that the method is exporting or transforming the SHA1 value into a different format.
///
/// These method naming conventions (`new`, `from`, `to`) provide clarity and predictability in the API, making it easier for users
/// to understand the intended use and functionality of each method within the `SHA1` struct.
impl SHA1 {
    // The size of the SHA-1 hash value in bytes
    pub const SIZE: usize = 20;

    /// Calculate the SHA-1 hash of the byte slice, then create a Hash value
    pub fn new(data: &[u8]) -> SHA1 {
        let h = sha1::Sha1::digest(data);
        SHA1::from_bytes(h.as_slice())
    }
    /// Create a Hash from the object type and data
    /// This function is used to create a SHA1 hash from the object type and data.
    /// It constructs a byte vector that includes the object type, the size of the data,
    /// and the data itself, and then computes the SHA1 hash of this byte vector.
    ///  
    ///  Hash compute <- {Object Type}+{ }+{Object Size（before compress）}+{\x00}+{Object Content(before compress)}
    pub fn from_type_and_data(object_type: ObjectType, data: &[u8]) -> SHA1 {
        let mut d: Vec<u8> = Vec::new();
        d.extend(object_type.to_data().unwrap());
        d.push(b' ');
        d.extend(data.len().to_string().as_bytes());
        d.push(b'\x00');
        d.extend(data);
        SHA1::new(&d)
    }

    /// Create Hash from a byte array, which is a 20-byte array already calculated
    pub fn from_bytes(bytes: &[u8]) -> SHA1 {
        let mut h = SHA1::default();
        h.0.copy_from_slice(bytes);
        h
    }

    /// Read the Hash value from the stream
    /// This function will read exactly 20 bytes from the stream
    pub fn from_stream(data: &mut impl io::Read) -> io::Result<SHA1> {
        let mut h = SHA1::default();
        data.read_exact(&mut h.0)?;
        Ok(h)
    }

    /// Export sha1 value to String with the color
    pub fn to_color_str(self) -> String {
        self.to_string().red().bold().to_string()
    }

    /// Export sha1 value to a byte array
    pub fn to_data(self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// [`core::fmt::Display`] is somewhat expensive,
    /// use this hack to get a string more efficiently
    pub fn _to_string(&self) -> String {
        hex::encode(self.0)
    }
}

#[cfg(test)]
mod tests {

    use std::io::BufReader;
    use std::io::Read;
    use std::io::Seek;
    use std::io::SeekFrom;
    use std::str::FromStr;
    use std::{env, path::PathBuf};

    use crate::hash::SHA1;

    #[test]
    fn test_sha1_new() {
        // Example input
        let data = "Hello, world!".as_bytes();

        // Generate SHA1 hash from the input data
        let sha1 = SHA1::new(data);

        // Known SHA1 hash for "Hello, world!"
        let expected_sha1_hash = "943a702d06f34599aee1f8da8ef9f7296031d699";

        assert_eq!(sha1.to_string(), expected_sha1_hash);
    }

    #[test]
    fn test_signature_without_delta() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/pack-1d0e6c14760c956c173ede71cb28f33d921e232f.pack");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);

        buffered.seek(SeekFrom::End(-20)).unwrap();
        let mut buffer = vec![0; 20];
        buffered.read_exact(&mut buffer).unwrap();
        let signature = SHA1::from_bytes(buffer.as_ref());
        assert_eq!(
            signature.to_string(),
            "1d0e6c14760c956c173ede71cb28f33d921e232f"
        );
    }

    #[test]
    fn test_sha1_from_bytes() {
        let sha1 = SHA1::from_bytes(&[
            0x8a, 0xb6, 0x86, 0xea, 0xfe, 0xb1, 0xf4, 0x47, 0x02, 0x73, 0x8c, 0x8b, 0x0f, 0x24,
            0xf2, 0x56, 0x7c, 0x36, 0xda, 0x6d,
        ]);

        assert_eq!(sha1.to_string(), "8ab686eafeb1f44702738c8b0f24f2567c36da6d");
    }

    #[test]
    fn test_from_stream() {
        let source = [
            0x8a, 0xb6, 0x86, 0xea, 0xfe, 0xb1, 0xf4, 0x47, 0x02, 0x73, 0x8c, 0x8b, 0x0f, 0x24,
            0xf2, 0x56, 0x7c, 0x36, 0xda, 0x6d,
        ];
        let mut reader = std::io::Cursor::new(source);
        let sha1 = SHA1::from_stream(&mut reader).unwrap();
        assert_eq!(sha1.to_string(), "8ab686eafeb1f44702738c8b0f24f2567c36da6d");
    }

    #[test]
    fn test_sha1_from_str() {
        let hash_str = "8ab686eafeb1f44702738c8b0f24f2567c36da6d";

        match SHA1::from_str(hash_str) {
            Ok(hash) => {
                assert_eq!(hash.to_string(), "8ab686eafeb1f44702738c8b0f24f2567c36da6d");
            }
            Err(e) => println!("Error: {e}"),
        }
    }

    #[test]
    fn test_sha1_to_string() {
        let hash_str = "8ab686eafeb1f44702738c8b0f24f2567c36da6d";

        match SHA1::from_str(hash_str) {
            Ok(hash) => {
                assert_eq!(hash.to_string(), "8ab686eafeb1f44702738c8b0f24f2567c36da6d");
            }
            Err(e) => println!("Error: {e}"),
        }
    }

    #[test]
    fn test_sha1_to_data() {
        let hash_str = "8ab686eafeb1f44702738c8b0f24f2567c36da6d";

        match SHA1::from_str(hash_str) {
            Ok(hash) => {
                assert_eq!(
                    hash.to_data(),
                    vec![
                        0x8a, 0xb6, 0x86, 0xea, 0xfe, 0xb1, 0xf4, 0x47, 0x02, 0x73, 0x8c, 0x8b,
                        0x0f, 0x24, 0xf2, 0x56, 0x7c, 0x36, 0xda, 0x6d
                    ]
                );
            }
            Err(e) => println!("Error: {e}"),
        }
    }
}
