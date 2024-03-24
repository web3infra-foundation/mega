use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use venus::errors::GitError;
use venus::hash::SHA1;
use crate::internal::model::utils;
use crate::internal::model::utils::SHA1_SIZE;
use crate::internal::pack::wrapper::Wrapper;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Time {
    seconds: u32,
    nanos: u32
}
impl Time {
    pub fn from_stream(stream: &mut impl Read) -> Result<Self, GitError> {
        let seconds = utils::read_u32_be(stream)?;
        let nanos = utils::read_u32_be(stream)?;
        Ok(Time { seconds, nanos })
    }

    fn to_system_time(&self) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::new(self.seconds.into(), self.nanos)
    }

    fn from_system_time(system_time: SystemTime) -> Self {
        match system_time.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let seconds = duration.as_secs().try_into().expect("Time is too far in the future");
                let nanos = duration.subsec_nanos();
                Time { seconds, nanos }
            }
            Err(_) => panic!("Time is before the UNIX epoch"),
        }
    }
}
impl Display for Time {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.seconds, self.nanos)
    }
}

/// 16 bits
#[derive(Debug)]
pub struct Flags {
    pub assume_valid: bool,
    pub extended: bool, // must be 0 in v2
    pub stage: u8, // 2-bit during merge
    pub name_length: u16 // 12-bit
}

impl Flags {
    pub fn new(flags: u16) -> Self {
        Flags {
            assume_valid: flags & 0x8000 != 0,
            extended: flags & 0x4000 != 0,
            stage: ((flags & 0x3000) >> 12) as u8,
            name_length: flags & 0xFFF
        }
    }

    pub fn to_u16(&self) -> u16 {
        let mut flags = 0u16;
        if self.assume_valid {
            flags |= 0x8000;
        }
        if self.extended {
            flags |= 0x4000;
        }
        flags |= (self.stage as u16) << 12;
        assert!(self.name_length <= 0xFFF, "Name length is too long");
        flags |= self.name_length;
        flags
    }
}

pub struct IndexEntry {
    pub ctime: Time,
    pub mtime: Time,
    pub dev: u32, // 0 for windows
    pub ino: u32, // 0 for windows
    pub mode: u32, // 0o100644
    pub uid: u32, // 0 for windows
    pub gid: u32, // 0 for windows
    pub size: u32,
    pub hash: SHA1,
    pub flags: Flags,
    pub name: String
}
impl Display for IndexEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IndexEntry {{ ctime: {}, mtime: {}, dev: {}, ino: {}, mode: {:o}, uid: {}, gid: {}, size: {}, hash: {}, flags: {:?}, name: {} }}",
               self.ctime, self.mtime, self.dev, self.ino, self.mode, self.uid, self.gid, self.size, self.hash, self.flags, self.name)
    }
}
/// see [index-format](https://git-scm.com/docs/index-format)
pub struct Index {
    entries: BTreeMap<PathBuf, IndexEntry>,
    work_dir: PathBuf
}

impl Index {
    fn check_header(file: &mut impl Read) -> Result<u32, GitError> {
        let mut magic = [0; 4];
        file.read_exact(&mut magic)?;
        if magic != *b"DIRC" {
            return Err(GitError::InvalidIndexHeader(String::from_utf8_lossy(&magic).to_string()));
        }

        let version = utils::read_u32_be(file)?;
        // only support v2 now
        if version != 2 {
            return Err(GitError::InvalidIndexHeader(version.to_string()));
        }

        let entries = utils::read_u32_be(file)?;
        Ok(entries)
    }

    pub fn new(work_dir: PathBuf) -> Self {
        Index {
            entries: BTreeMap::new(),
            work_dir
        }
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    pub fn from_file(path: impl AsRef<Path>, work_dir: &Path) -> Result<Self, GitError> {
        let file = File::open(path.as_ref())?; // read-only
        let total_size = file.metadata()?.len();
        let file = &mut Wrapper::new(BufReader::new(file)); // TODO move Wrapper & utils to a common module

        let num = Index::check_header(file)?;
        let mut index = Index::new(work_dir.to_path_buf());

        println!("number: {}", num);
        for _ in 0..num {
            let mut entry = IndexEntry {
                ctime: Time::from_stream(file)?,
                mtime: Time::from_stream(file)?,
                dev: utils::read_u32_be(file)?,
                ino: utils::read_u32_be(file)?,
                mode: utils::read_u32_be(file)?,
                uid: utils::read_u32_be(file)?,
                gid: utils::read_u32_be(file)?,
                size: utils::read_u32_be(file)?,
                hash: utils::read_sha1(file)?,
                flags: Flags::new(utils::read_u16_be(file)?),
                name: String::new()
            };
            let name_len = entry.flags.name_length as usize;
            let mut name = vec![0; name_len];
            file.read_exact(&mut name)?;
            // The exact encoding is undefined, but the '.' and '/' characters are encoded in 7-bit ASCII
            entry.name = String::from_utf8(name)?; // TODO check the encoding
            index.entries.insert(PathBuf::from(entry.name.clone()), entry); // TODO determine relative or absolute path

            // 1-8 nul bytes as necessary to pad the entry to a multiple of eight bytes
            // while keeping the name NUL-terminated. // so at least 1 byte nul
            let padding = 8 - ((22 + name_len) % 8); // 22 = sha1 + flags, others are 40 % 8 == 0
            utils::read_bytes(file, padding)?;
        }

        // Extensions
        while file.bytes_read() + SHA1_SIZE < total_size as usize { // The remaining 20 bytes must be checksum
            let sign = utils::read_bytes(file, 4)?;
            println!("{:?}", String::from_utf8(sign.clone())?);
            // If the first byte is 'A'...'Z' the extension is optional and can be ignored.
            if sign[0] >= ('A' as u8) && sign[0] <= ('Z' as u8) { // Optional extension
                let size = utils::read_u32_be(file)?;
                utils::read_bytes(file, size as usize)?; // Ignore the extension
            } else { // 'link' or 'sdir' extension
                return Err(GitError::InvalidIndexFile("Unsupported extension".to_string()));
            }
        }

        // check sum
        let file_hash = file.final_hash();
        let check_sum = utils::read_sha1(file)?;
        if file_hash != check_sum {
            return Err(GitError::InvalidIndexFile("Check sum failed".to_string()));
        }
        assert_eq!(index.size(), num as usize);
        Ok(index)
    }
}

mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;
    use crate::internal::model::index::{Index, Time};

    #[test]
    fn test_time() {
        let time = Time { seconds: 0, nanos: 0 };
        let system_time = time.to_system_time();
        let new_time = Time::from_system_time(system_time);
        assert_eq!(time, new_time);
    }

    #[test]
    fn test_check_header() {
        let file = File::open("../tests/data/index/index-2").unwrap();
        let entries = Index::check_header(&mut BufReader::new(file)).unwrap();
        assert_eq!(entries, 2);
    }

    #[test]
    fn test_index() {
        let index = Index::from_file("../tests/data/index/index-760", Path::new("")).unwrap();
        assert_eq!(index.size(), 760);
        for (_, entry) in index.entries.iter() {
            println!("{}", entry);
        }
    }
}