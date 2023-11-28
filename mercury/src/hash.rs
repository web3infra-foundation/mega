//! In Git, the SHA-1 hash algorithm is widely used to generate unique identifiers for Git objects.
//! Each Git object corresponds to a unique SHA-1 hash value, which is used to identify the object's
//! location in the Git database.
//!

use std::fmt::Display;

use colored::Colorize;
use sha1_smol::Digest;
use serde::{Deserialize, Serialize};

/// The Hash struct which only contain the u8 array :`[u8;20]` is used to represent Git hash IDs,
/// which are 40-character hexadecimal strings computed using the SHA-1 algorithm. In Git, each object
/// is assigned a unique hash ID based on its content, which is used to identify
/// the object's location in the Git database.The Hash struct provides a convenient
/// way to store and manipulate Git hash IDs by using a separate struct for hash IDs to make
/// code more readable and maintainable.
#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default,Deserialize, Serialize)]
pub struct SHA1(pub [u8; 20]);

/// Display trait for Hash type
impl Display for SHA1 {
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

impl std::str::FromStr for SHA1 {
    type Err = &'static str;

    /// Create Hash from a string, which is a 40-character hexadecimal string already calculated
    fn from_str(s: &str) ->  Result<Self, Self::Err> {
        let mut h = SHA1::default();

        let d = Digest::from_str(s);

        match d {
            Ok(d) => h.0.copy_from_slice(d.bytes().as_slice()),  
            Err(_e) => return Err("Hash from string encounter error"),
        }

        Ok(h)
    }
}

impl SHA1 {
    /// Calculate the SHA-1 hash of `Vec<u8>` data, then create a Hash value
    pub fn new(data: &Vec<u8>) -> SHA1 {
        // Create a Sha1 object for calculating the SHA-1 hash
        let s = sha1_smol::Sha1::from(data);
        // Get the result of the hash
        let sha1 = s.digest();
        // Convert the result to a 20-byte array
        let result = sha1.bytes();

        SHA1(result)
    }

    /// Create Hash from a byte array, which is a 20-byte array already calculated
    pub fn from_bytes(bytes: &[u8]) -> SHA1 {
        let mut h = SHA1::default();
        h.0.copy_from_slice(bytes);
        
        h
    }

    /// Export sha1 value to plain String without the color chars
    pub fn to_plain_str(self) -> String {
        hex::encode(self.0)
    }

    /// Export sha1 value to a byte array
    pub fn to_data(self) -> Vec<u8> {
        self.0.to_vec()
    }

}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::hash::SHA1;

    #[test]
    fn test_hash_new() {
        let hash = SHA1::from_bytes(&[
            0x8a, 0xb6, 0x86, 0xea, 0xfe, 0xb1, 0xf4, 0x47, 0x02, 0x73, 0x8c, 0x8b, 0x0f, 0x24,
            0xf2, 0x56, 0x7c, 0x36, 0xda, 0x6d,
        ]);
        assert_eq!(
            hash.to_plain_str(),
            "8ab686eafeb1f44702738c8b0f24f2567c36da6d"
        );
    }

    #[test]
    fn test_hash_from_bytes() {
        let hash = SHA1::from_bytes(&[
            0x8a, 0xb6, 0x86, 0xea, 0xfe, 0xb1, 0xf4, 0x47, 0x02, 0x73, 0x8c, 0x8b, 0x0f, 0x24,
            0xf2, 0x56, 0x7c, 0x36, 0xda, 0x6d,
        ]);
        assert_eq!(
            hash.to_plain_str(),
            "8ab686eafeb1f44702738c8b0f24f2567c36da6d"
        );
    }

    #[test]
    fn test_hash_from_str() {
        let hash_str = "8ab686eafeb1f44702738c8b0f24f2567c36da6d";

        match SHA1::from_str(hash_str) {
            Ok(hash) => {
                assert_eq!(
                    hash.to_plain_str(), "8ab686eafeb1f44702738c8b0f24f2567c36da6d");
            },
            Err(e) => println!("Error: {}", e),
        }
    }

    #[test]
    fn test_hash_to_data() {
        let hash_str = "8ab686eafeb1f44702738c8b0f24f2567c36da6d";

        match SHA1::from_str(hash_str) {
            Ok(hash) => {
                assert_eq!(
                    hash.to_data(),
                    vec![
                        0x8a, 0xb6, 0x86, 0xea, 0xfe, 0xb1, 0xf4, 0x47, 0x02, 0x73, 0x8c, 0x8b, 0x0f, 0x24,
                        0xf2, 0x56, 0x7c, 0x36, 0xda, 0x6d
                    ]
                );
            },
            Err(e) => println!("Error: {}", e),
            
        }
    }


}
