//! In Git, the SHA-1 hash algorithm is widely used to generate unique identifiers for Git objects.
//! Each Git object corresponds to a unique SHA-1 hash value, which is used to identify the object's
//! location in the Git database.
//!

use std::fmt::Display;

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
        let hash_re = hasher.finalize();
        // Convert the result to a 20-byte array
        let result = <[u8; 20]>::from(hash_re);

        Hash(result)
    }

    /// Create plain String without the color chars
    #[allow(unused)]
    pub fn to_plain_str(self) -> String {
        hex::encode(self.0)
    }
}

mod tests {
    #[test]
    fn test_hash_new() {
        let hash = super::Hash::new(&vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
        assert_eq!(hash.to_plain_str(), "e89ad5a9631c3efdded7e3ecce79b4d0fedce1bf");
    }
}