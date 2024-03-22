use std::fmt::{Display, Formatter};
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};
use venus::errors::GitError;
use venus::hash::SHA1;
use crate::internal::model::utils;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Time {
    seconds: u32,
    nanos: u32
}
impl Time {
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
    pub flags: u16,
    pub name: String
}

pub struct Index {

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
}

mod tests {
    use std::fs::File;
    use std::io::BufReader;
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
}