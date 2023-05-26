use std::io::{Read, Seek};

use crate::{errors::GitError, hash::Hash, utils};

use super::Pack;

impl Pack {
    /// Git [Pack Format](https://github.com/git/git/blob/master/Documentation/technical/pack-format.txt)
    /// Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
    /// ## Decode the Pack File without the `.idx` File
    ///  - in: pack_file: &mut File
    ///  - out: The `Pack` Struct
    pub async fn decode(mut pack_file: impl Read + Send + Seek) -> Result<Self, GitError> {
        // Check the Header of Pack File
        let mut pack = Self::check_header(&mut pack_file)?;
        // TODO add decode process
        // CheckSum sha-1
        let id: [u8; 20] = utils::read_bytes(&mut pack_file).unwrap();
        pack.signature = Hash::new_from_bytes(&id[..]);

        Ok(pack)
    }

    /// Check the Header of the Pack File ,<br>
    /// include the **"PACK" head** , **Version Number** and  **Number of the Objects**
    fn check_header(mut pack_file: impl Read + Send) -> Result<Self, GitError> {
        //init a Pack Struct ,which is all empty
        let mut pack = Self::default();

        // Get the Pack Head 4 b ,which should be the "PACK"
        let magic = utils::read_bytes(&mut pack_file).unwrap();
        if magic != *b"PACK" {
            return Err(GitError::InvalidPackHeader(format!(
                "{},{},{},{}",
                magic[0], magic[1], magic[2], magic[3]
            )));
        }
        pack.head = magic;

        //Get the Version Number
        let version = utils::read_u32(&mut pack_file).unwrap();
        if version != 2 {
            return Err(GitError::InvalidPackFile("Current File".to_string()));
        }
        pack.version = version;

        let object_num = utils::read_u32(&mut pack_file).unwrap();
        pack.number_of_objects = object_num as usize;

        Ok(pack)
    }
}
