//! ## This is a temp Pack for developing and will merge to git library 
//! in complete the all functions.
//! 
//!
//!
use std::sync::Arc;
use std::io::Read;


use common::utils;

use git::hash::Hash;
use git::internal::object::ObjectT;
use git::errors::GitError;

use crate::cache::Cache;

#[allow(unused)]
pub struct Pack {
    pub number: usize,
    pub signature: Hash,
    pub objects: Box<dyn Cache<T = Arc<dyn ObjectT>>>,
}

impl Pack {
    /// Check the Header of the Pack,<br>
    /// include the **"PACK" head** , **Version Number** and  **Number of the Objects**
    /// and return the number of the objects
    pub fn check_header(pack: &mut impl Read) -> Result<usize, GitError> {
        // Get the Pack Head 4 b ,which should be the "PACK"
        let magic = utils::read_bytes(pack).unwrap();

        if magic != *b"PACK" {
            return Err(GitError::InvalidPackHeader(format!(
                "{},{},{},{}",
                magic[0], magic[1], magic[2], magic[3]
            )));
        }

        //Get the version Number
        let version = utils::read_u32(pack).unwrap();
        if version != 2 {
            return Err(GitError::InvalidPackFile("Current file version is not 2".to_string()));
        }

        //Get the number of the Objects
        let object_num = utils::read_u32(pack).unwrap();
        
        Ok(object_num as usize)
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, env};

    #[test]
    fn test_read_pack_from_file() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git.pack");
    }
}