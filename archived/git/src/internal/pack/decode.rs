use std::io::{self, BufRead};
use std::io::{Read, Seek};

use sha1::digest::core_api::CoreWrapper;
use sha1::Digest;
use sha1::Sha1;

use crate::hash::Hash;
use crate::internal::pack::{iterator::EntriesIter, Pack};
use crate::{errors::GitError, utils};
#[allow(unused)]
enum DecodeMod {
    Plain,
    HashCount,
}

impl Pack {
    /// Git [Pack Format](https://github.com/git/git/blob/master/Documentation/technical/pack-format.txt)
    /// Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
    /// ## Decode the Pack File without the `.idx` File
    ///  - in: pack_file: &mut impl Read + Seek + Send
    ///  - out: The `Pack` Struct
    pub async fn decode(pack_file: &mut (impl Read + Seek + Send)) -> Result<Self, GitError> {
        // change this to input ?
        // let mode = DecodeMod::HashCount;
        // match mode {
        //     DecodeMod::Plain => {
        //         count_hash= false;
        //     },
        //     DecodeMod::HashCount => {
        //         count_hash=true;
        //     },
        // }
        let count_hash: bool = true;
        let mut reader = HashCounter::new(io::BufReader::new(pack_file), count_hash);
        // Read the header of the pack file
        let mut pack = Pack::check_header(&mut reader)?;

        let mut iterator = EntriesIter::new(&mut reader, pack.number_of_objects as u32);
        for _ in 0..pack.number_of_objects {
            let obj = iterator.next_obj().await?;
            println!("{}", obj);
        }
        drop(iterator);

        // Compute the Checksum Hash value of all pack file.
        // Read the Checksum Hash form the Pack stream tail.
        // Check if the two are consistent.
        let _hash = reader.final_hash();
        pack.signature = read_tail_hash(&mut reader);
        assert_eq!(_hash, pack.signature);

        Ok(pack)
    }

    /// Check the Header of the Pack File ,<br>
    /// include the **"PACK" head** , **Version Number** and  **Number of the Objects**
    pub fn check_header(pack_file: &mut impl Read) -> Result<Self, GitError> {
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
/// A BufReader for hash count during the pack data stream "read".
pub struct HashCounter<R> {
    inner: R,
    hash: CoreWrapper<sha1::Sha1Core>,
    count_hash: bool,
}
impl<R> HashCounter<R>
where
    R: BufRead,
{
    pub fn new(inner: R, count_hash: bool) -> Self {
        Self {
            inner,
            hash: Sha1::new(),
            count_hash,
        }
    }
    pub fn final_hash(&self) -> Hash {
        let re: [u8; 20] = self.hash.clone().finalize().into();
        Hash(re)
    }
}
impl<R> BufRead for HashCounter<R>
where
    R: BufRead,
{
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }
    /// Count the Hash : Update the hash core value by consume's amt
    fn consume(&mut self, amt: usize) {
        let buffer = self.inner.fill_buf().expect("Failed to fill buffer");
        if self.count_hash {
            self.hash.update(&buffer[..amt]);
        }
        self.inner.consume(amt);
    }
}
impl<R> Read for HashCounter<R>
where
    R: BufRead,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let o = self.inner.read(buf)?;
        if self.count_hash {
            self.hash.update(&buf[..o]);
        }
        Ok(o)
    }
}

fn read_tail_hash(tail: &mut impl Read) -> Hash {
    let id: [u8; 20] = {
        let mut id_buf = [0u8; 20];
        tail.read_exact(&mut id_buf).unwrap();
        id_buf
    };
    Hash::new_from_bytes(&id[..])
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
