use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use venus::errors::GitError;
use venus::hash::SHA1;
// use utils;
// use utils::SHA1_SIZE;
use mercury::internal::pack::wrapper::Wrapper;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Time {
    seconds: u32,
    nanos: u32,
}
impl Time {
    #[allow(dead_code)]
    pub fn from_stream(stream: &mut impl Read) -> Result<Self, GitError> {
        let seconds = stream.read_u32::<BigEndian>()?;
        let nanos = stream.read_u32::<BigEndian>()?;
        Ok(Time { seconds, nanos })
    }

    #[allow(dead_code)]
    fn to_system_time(&self) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::new(self.seconds.into(), self.nanos)
    }

    #[allow(dead_code)]
    fn from_system_time(system_time: SystemTime) -> Self {
        match system_time.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let seconds = duration
                    .as_secs()
                    .try_into()
                    .expect("Time is too far in the future");
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
    pub extended: bool,   // must be 0 in v2
    pub stage: u8,        // 2-bit during merge
    pub name_length: u16, // 12-bit
}

impl Flags {
    pub fn new(flags: u16) -> Self {
        Flags {
            assume_valid: flags & 0x8000 != 0,
            extended: flags & 0x4000 != 0,
            stage: ((flags & 0x3000) >> 12) as u8,
            name_length: flags & 0xFFF,
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
    pub dev: u32,  // 0 for windows
    pub ino: u32,  // 0 for windows
    pub mode: u32, // 0o100644 // 4-bit object type + 3-bit unused + 9-bit unix permission
    pub uid: u32,  // 0 for windows
    pub gid: u32,  // 0 for windows
    pub size: u32,
    pub hash: SHA1,
    pub flags: Flags,
    pub name: String,
}
impl Display for IndexEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IndexEntry {{ ctime: {}, mtime: {}, dev: {}, ino: {}, mode: {:o}, uid: {}, gid: {}, size: {}, hash: {}, flags: {:?}, name: {} }}",
               self.ctime, self.mtime, self.dev, self.ino, self.mode, self.uid, self.gid, self.size, self.hash, self.flags, self.name)
    }
}

impl IndexEntry {
    pub fn new(meta: &fs::Metadata, hash: SHA1, name: String) -> Self {
        let mut entry = IndexEntry {
            ctime: Time::from_system_time(meta.created().unwrap()),
            mtime: Time::from_system_time(meta.modified().unwrap()),
            dev: 0,
            ino: 0,
            uid: 0,
            gid: 0,
            size: meta.len() as u32,
            hash,
            flags: Flags::new(0),
            name,
            mode: 0o100644,
        };
        #[cfg(unix)]
        {
            entry.dev = meta.dev() as u32;
            entry.ino = meta.ino() as u32;
            entry.uid = meta.uid();
            entry.gid = meta.gid();

            entry.mode = match meta.mode() & 0o170000/* file mode */ {
                0o100000 => {
                    match meta.mode() & 0o111 {
                        0 => 0o100644, // no execute permission
                        _ => 0o100755, // with execute permission
                    }
                }
                0o120000 => 0o120000, // symlink
                _ =>  entry.mode, // keep the original mode
            }
        }
        entry
    }
}

/// see [index-format](https://git-scm.com/docs/index-format)
/// <br> to Working Dir relative path
pub struct Index {
    entries: BTreeMap<(String, u8), IndexEntry>,
}

impl Index {
    fn check_header(file: &mut impl Read) -> Result<u32, GitError> {
        let mut magic = [0; 4];
        file.read_exact(&mut magic)?;
        if magic != *b"DIRC" {
            return Err(GitError::InvalidIndexHeader(
                String::from_utf8_lossy(&magic).to_string(),
            ));
        }

        let version = file.read_u32::<BigEndian>()?;
        // only support v2 now
        if version != 2 {
            return Err(GitError::InvalidIndexHeader(version.to_string()));
        }

        let entries = file.read_u32::<BigEndian>()?;
        Ok(entries)
    }

    pub fn new() -> Self {
        Index {
            entries: BTreeMap::new(),
        }
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, GitError> {
        let file = File::open(path.as_ref())?; // read-only
        let total_size = file.metadata()?.len();
        let file = &mut Wrapper::new(BufReader::new(file)); // TODO move Wrapper & utils to a common module

        let num = Index::check_header(file)?;
        let mut index = Index::new();

        println!("number: {}", num);
        for _ in 0..num {
            let mut entry = IndexEntry {
                ctime: Time::from_stream(file)?,
                mtime: Time::from_stream(file)?,
                dev: file.read_u32::<BigEndian>()?, //utils::read_u32_be(file)?,
                ino: file.read_u32::<BigEndian>()?,
                mode: file.read_u32::<BigEndian>()?,
                uid: file.read_u32::<BigEndian>()?,
                gid: file.read_u32::<BigEndian>()?,
                size: file.read_u32::<BigEndian>()?,
                hash: utils::read_sha1(file)?,
                flags: Flags::new(file.read_u16::<BigEndian>()?),
                name: String::new(),
            };
            let name_len = entry.flags.name_length as usize;
            let mut name = vec![0; name_len];
            file.read_exact(&mut name)?;
            // The exact encoding is undefined, but the '.' and '/' characters are encoded in 7-bit ASCII
            entry.name = String::from_utf8(name)?; // TODO check the encoding
            index
                .entries
                .insert((entry.name.clone(), entry.flags.stage), entry);

            // 1-8 nul bytes as necessary to pad the entry to a multiple of eight bytes
            // while keeping the name NUL-terminated. // so at least 1 byte nul
            let padding = 8 - ((22 + name_len) % 8); // 22 = sha1 + flags, others are 40 % 8 == 0
            utils::read_bytes(file, padding)?;
        }

        // Extensions
        while file.bytes_read() + utils::SHA1_SIZE < total_size as usize {
            // The remaining 20 bytes must be checksum
            let sign = utils::read_bytes(file, 4)?;
            println!("{:?}", String::from_utf8(sign.clone())?);
            // If the first byte is 'A'...'Z' the extension is optional and can be ignored.
            if sign[0] >= b'A' && sign[0] <= b'Z' {
                // Optional extension
                let size = file.read_u32::<BigEndian>()?;
                utils::read_bytes(file, size as usize)?; // Ignore the extension
            } else {
                // 'link' or 'sdir' extension
                return Err(GitError::InvalidIndexFile(
                    "Unsupported extension".to_string(),
                ));
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

    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<(), GitError> {
        let mut file = File::create(path)?;
        let mut hash = Sha1::new();

        let mut header = Vec::new();
        header.write_all(b"DIRC")?;
        header.write_u32::<BigEndian>(2u32)?; // version 2
        header.write_u32::<BigEndian>(self.entries.len() as u32)?;
        file.write_all(&header)?;
        hash.update(&header);

        for (_, entry) in self.entries.iter() {
            let mut entry_bytes = Vec::new();
            entry_bytes.write_u32::<BigEndian>(entry.ctime.seconds)?;
            entry_bytes.write_u32::<BigEndian>(entry.ctime.nanos)?;
            entry_bytes.write_u32::<BigEndian>(entry.mtime.seconds)?;
            entry_bytes.write_u32::<BigEndian>(entry.mtime.nanos)?;
            entry_bytes.write_u32::<BigEndian>(entry.dev)?;
            entry_bytes.write_u32::<BigEndian>(entry.ino)?;
            entry_bytes.write_u32::<BigEndian>(entry.mode)?;
            entry_bytes.write_u32::<BigEndian>(entry.uid)?;
            entry_bytes.write_u32::<BigEndian>(entry.gid)?;
            entry_bytes.write_u32::<BigEndian>(entry.size)?;
            entry_bytes.write_all(&entry.hash.0)?;
            entry_bytes.write_u16::<BigEndian>(entry.flags.to_u16())?;
            entry_bytes.write_all(entry.name.as_bytes())?;
            let padding = 8 - ((22 + entry.name.len()) % 8);
            entry_bytes.write_all(&vec![0; padding])?;

            file.write_all(&entry_bytes)?;
            hash.update(&entry_bytes);
        }

        // Extensions

        // check sum
        let file_hash: [u8; 20] = hash.finalize().into();
        file.write_all(&file_hash)?;
        Ok(())
    }
}

impl Index {
    pub fn update(&mut self, entry: IndexEntry) {
        self.entries
            .insert((entry.name.clone(), entry.flags.stage), entry);
    }

    pub fn remove(&mut self, name: &str, stage: u8) {
        self.entries.remove(&(name.to_string(), stage));
    }

    pub fn get(&self, name: &str, stage: u8) -> Option<&IndexEntry> {
        self.entries.get(&(name.to_string(), stage))
    }

    pub fn tracked(&self, name: &str) -> bool {
        self.entries.contains_key(&(name.to_string(), 0))
    }
}

mod utils {
    use std::io;
    use std::io::Read;
    use venus::hash::SHA1;

    pub const SHA1_SIZE: usize = 20;

    pub fn read_bytes(file: &mut impl Read, len: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; len];
        file.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_sha1(file: &mut impl Read) -> io::Result<SHA1> {
        let mut buf = [0; 20];
        file.read_exact(&mut buf)?;
        Ok(SHA1::from_bytes(&buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_time() {
        let time = Time {
            seconds: 0,
            nanos: 0,
        };
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
        let index = Index::from_file("../tests/data/index/index-760").unwrap();
        assert_eq!(index.size(), 760);
        for (_, entry) in index.entries.iter() {
            println!("{}", entry);
        }
    }

    #[test]
    fn test_index_to_file() {
        let index = Index::from_file("../tests/data/index/index-760").unwrap();
        index.to_file("/tmp/index-760").unwrap();
        let new_index = Index::from_file("/tmp/index-760").unwrap();
        assert_eq!(index.size(), new_index.size());
    }

    #[test]
    fn test_index_entry_create() {
        // test create index entry from file
        let file = PathBuf::from("../tests/integration_test.rs"); // use as a normal file
        let meta = file.metadata().unwrap();
        let hash = SHA1::from_bytes(&[0; 20]);
        let name = file.as_os_str().to_str().unwrap().to_string();
        let entry = IndexEntry::new(&meta, hash, name);
        println!("{}", entry);
    }
}
