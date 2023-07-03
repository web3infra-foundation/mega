use std::io;
use std::io::{Read, Seek};

use crate::{errors::GitError, utils};

use super::{iterator::EntriesIter, Pack};

impl Pack {
    /// Git [Pack Format](https://github.com/git/git/blob/master/Documentation/technical/pack-format.txt)
    /// Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
    /// ## Decode the Pack File without the `.idx` File
    ///  - in: pack_file: &mut impl Read + Seek + Send
    ///  - out: The `Pack` Struct

    pub async fn decode(mut pack_file: &mut (impl Read + Seek + Send)) -> Result<Self, GitError> {
        // Check the Header of Pack File
        let mut pack = Pack::check_header(pack_file)?;

        let obj_total = pack.number_of_objects();

        let mut inter = EntriesIter::new(
            io::BufReader::with_capacity(4096, &mut pack_file),
            obj_total as u32,
        );
        for _ in 0..obj_total {
            let obj = inter.next_obj().await?;
            println!("{}", obj);
        }

        //pack.signature = Hash::new_from_bytes(&id[..]);
        pack.signature = inter.read_tail_hash();

        Ok(pack)
    }

    /// Check the Header of the Pack File ,<br>
    /// include the **"PACK" head** , **Version Number** and  **Number of the Objects**
    fn check_header(pack_file: &mut impl Read) -> Result<Self, GitError> {
        //init a Pack Struct ,which is all empty
        let mut pack = Self::default();

        // Get the Pack Head 4 b ,which should be the "PACK"
        let magic = utils::read_bytes(pack_file).unwrap();
        if magic != *b"PACK" {
            return Err(GitError::InvalidPackHeader(format!(
                "{},{},{},{}",
                magic[0], magic[1], magic[2], magic[3]
            )));
        }
        pack.head = magic;

        //Get the Version Number
        let version = utils::read_u32(pack_file).unwrap();
        if version != 2 {
            return Err(GitError::InvalidPackFile("Current File".to_string()));
        }
        pack.version = version;

        let object_num = utils::read_u32(pack_file).unwrap();
        pack.number_of_objects = object_num as usize;

        Ok(pack)
    }
}
#[cfg(test)]
mod test {
    use std::{fs::File, path::Path};

    use tokio_test::block_on;

    use crate::internal::pack::Pack;

    #[test]
    fn test_async_buffer() {
        let mut file = File::open(Path::new(
            "../tests/data/packs/pack-d50df695086eea6253a237cb5ac44af1629e7ced.pack",
        ))
        .unwrap();

        let p = block_on(Pack::decode(&mut file)).unwrap();
        assert_eq!(p.version, 2);
        assert_eq!(p.number_of_objects, p.number_of_objects());
    }

    #[test]

    fn test_async_buffer2() {
        let mut file = File::open(Path::new(
            "../tests/data/packs/pack-1d0e6c14760c956c173ede71cb28f33d921e232f.pack",
        ))
        .unwrap();
        let p = block_on(Pack::decode(&mut file)).unwrap();
        assert_eq!(p.version, 2);
        assert_eq!(p.number_of_objects, p.number_of_objects());
    }
}
