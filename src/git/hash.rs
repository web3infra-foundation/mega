//! In Git, the SHA-1 hash algorithm is widely used to generate unique identifiers for Git objects.
//! Each Git object corresponds to a unique SHA-1 hash value, which is used to identify the object's
//! location in the Git database.
//!

use std::fmt::Display;

use bstr::ByteSlice;
use colored::Colorize;
use sha1::{Digest, Sha1};

/// The Hash struct which only contain the u8 array :`[u8;20]` is used to represent Git hash IDs,
/// which are 40-character hexadecimal strings computed using the SHA-1 algorithm. In Git, each object
/// is assigned a unique hash ID based on its content, which is used to identify
/// the object's location in the Git database.The Hash struct provides a convenient
/// way to store and manipulate Git hash IDs by using a separate struct for hash IDs to make
/// code more readable and maintainable.
#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Hash(pub [u8; 20]);

/// Display trait for Hash type
impl Display for Hash {
    /// # Attention
    /// cause of the color chars for ,if you want to use the string without color ,
    /// please call the func:`to_plain_str()` rather than the func:`to_string()`
    /// # Example
    ///  the hash value `18fd2deaaf152c7f1222c52fb2673f6192b375f0`<br>
    ///  will be the `1;31m8d2deaaf152c7f1222c52fb2673f6192b375f00m`
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_plain_str().red().bold())
    }
}

impl Hash {
    /// Calculate the SHA-1 hash of `Vec<u8>` data
    /// # Example
    /// ```
    /// let hash = Hash::new(&vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
    /// assert_eq!(hash.to_plain_str(), "e89ad5a9631c3efdded7e3ecce79b4d0fedce1bf");
    /// ```
    #[allow(unused)]
    pub fn new(data: &Vec<u8>) -> Hash {
        // Create a Sha1 object for calculating the SHA-1 hash
        let mut hasher = Sha1::new();
        // Input the data into the Sha1 object
        hasher.update(data);
        // Get the result of the hash
        let hash_result = hasher.finalize();
        // Convert the result to a 20-byte array
        let result = <[u8; 20]>::from(hash_result);

        Hash(result)
    }

    /// Create Hash from a byte array
    #[allow(unused)]
    pub fn new_from_bytes(bytes: &[u8]) -> Hash {
        let mut h = Hash::default();
        h.0.copy_from_slice(bytes);
        h
    }

    /// Create Hash from a string, which is a 40-character hexadecimal string
    #[allow(unused)]
    pub fn new_from_str(s: &str) -> Hash {
        let mut h = Hash::default();
        h.0.copy_from_slice(&hex::decode(s).unwrap());
        h
    }

    /// Create plain String without the color chars
    #[allow(unused)]
    pub fn to_plain_str(self) -> String {
        hex::encode(self.0)
    }

    #[allow(unused)]
    pub fn to_data(self) -> Vec<u8> {
        self.0.repeatn(1)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_hash_new() {
        // [98, 108, 111, 98] = blob
        // [32] = Space
        // [49, 52] = 14
        // [0] = \x00
        // [72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33, 10] = Hello, World! + LF
        let hash =
            super::Hash::new(&vec![98, 108, 111, 98, 32, 49, 52, 0, 72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33, 10]);
        assert_eq!(hash.to_plain_str(), "8ab686eafeb1f44702738c8b0f24f2567c36da6d");
    }

    #[test]
    fn test_hash_new_from_str() {
        let hash = super::Hash::new_from_str("8ab686eafeb1f44702738c8b0f24f2567c36da6d");
        assert_eq!(hash.to_plain_str(), "8ab686eafeb1f44702738c8b0f24f2567c36da6d");
    }

    #[test]
    fn test_hash_to_data() {
        let hash = super::Hash::new_from_str("8ab686eafeb1f44702738c8b0f24f2567c36da6d");
        assert_eq!(hash.to_data(), vec![0x8a, 0xb6, 0x86, 0xea, 0xfe, 0xb1, 0xf4, 0x47, 0x02, 0x73, 0x8c, 0x8b, 0x0f, 0x24, 0xf2, 0x56, 0x7c, 0x36, 0xda, 0x6d]);
    }

    #[test]
    fn test_hash_from_bytes() {
        let hash = super::Hash::new_from_bytes(&vec![0x8a, 0xb6, 0x86, 0xea, 0xfe, 0xb1, 0xf4, 0x47, 0x02, 0x73, 0x8c, 0x8b, 0x0f, 0x24, 0xf2, 0x56, 0x7c, 0x36, 0xda, 0x6d]);
        assert_eq!(hash.to_plain_str(), "8ab686eafeb1f44702738c8b0f24f2567c36da6d");
    }
}