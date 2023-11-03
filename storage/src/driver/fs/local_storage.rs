use common::errors::MegaError;
use sha256::digest;
use std::fs;
use std::io::prelude::*;
use std::path;
use std::path::PathBuf;

use super::FileStorage;

pub struct LocalStorage {
    base_path: PathBuf,
}

#[derive(Debug, Default)]
pub struct MetaObject {
    pub oid: String,
    pub size: i64,
    pub exist: bool,
}

impl LocalStorage {
    pub fn init(base: PathBuf) -> LocalStorage {
        fs::create_dir_all(&base).expect("Create directory failed!");
        LocalStorage { base_path: base }
    }
}

impl FileStorage for LocalStorage {
    fn get(&self, object_id: &str) -> fs::File {
        let path = path::Path::new(&self.base_path).join(Self::transform_path(object_id));
        fs::File::open(path).expect("Open file failed!")
    }

    fn put(&self, object_id: &str, size: i64, body_content: &[u8]) -> Result<String, MegaError> {
        let path = path::Path::new(&self.base_path).join(Self::transform_path(object_id));
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir).expect("Create directory failed!");

        let mut file = fs::File::create(&path).expect("Open file failed");
        let lenght_written = file.write(body_content).expect("Write file failed");
        if lenght_written as i64 != size {
            return Err(MegaError::with_message("size not correct"));
        }
        let hash = digest(body_content);
        if hash != object_id {
            return Err(MegaError::with_message("hash not matched"));
        }
        Ok(path.to_str().unwrap().to_string())
    }

    fn exist(&self, object_id: &str) -> bool {
        let path = path::Path::new(&self.base_path).join(Self::transform_path(object_id));

        path::Path::exists(&path)
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn test_content_store() {
        let meta = MetaObject {
            oid: "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72".to_owned(),
            size: 12,
            exist: false,
        };

        let content = "test content".as_bytes();

        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/objects");

        let local_storage = LocalStorage::init(source.clone());
        assert!(local_storage.put(&meta.oid, meta.size, content).is_ok());

        assert!(local_storage.exist(&meta.oid));
    }
}
