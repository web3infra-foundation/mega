use std::fs;
use std::io::prelude::*;
use std::path;
use std::path::PathBuf;

use async_trait::async_trait;
use bytes::Bytes;

use common::errors::MegaError;

use crate::driver::file_storage::FileStorage;

#[derive(Default)]
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
    pub fn init(base_path: PathBuf) -> LocalStorage {
        fs::create_dir_all(&base_path).expect("Create directory failed!");
        LocalStorage { base_path }
    }
}

#[async_trait]
impl FileStorage for LocalStorage {
    async fn get(&self, object_id: &str) -> Result<Bytes, MegaError> {
        let path = path::Path::new(&self.base_path).join(self.transform_path(object_id));
        let mut file = fs::File::open(&path).unwrap_or_else(|_| panic!("Open file:{:?} failed!", path));
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        Ok(Bytes::from(buffer))
    }

    async fn put(
        &self,
        object_id: &str,
        size: i64,
        body_content: &[u8],
    ) -> Result<String, MegaError> {
        let path = path::Path::new(&self.base_path).join(self.transform_path(object_id));
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir).expect("Create directory failed!");

        let mut file = fs::File::create(&path).expect("Open file failed");
        let lenght_written = file.write(body_content).expect("Write file failed");
        if lenght_written as i64 != size {
            return Err(MegaError::with_message("size not correct"));
        }
        Ok(path.to_str().unwrap().to_string())
    }

    fn exist(&self, object_id: &str) -> bool {
        let path = path::Path::new(&self.base_path).join(self.transform_path(object_id));

        path::Path::exists(&path)
    }
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use crate::driver::file_storage::{local_storage::{MetaObject, LocalStorage}, FileStorage};

    // #[test]
    #[tokio::test]
    async fn test_content_store() {
        let meta = MetaObject {
            oid: "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72".to_owned(),
            size: 12,
            exist: false,
        };

        let content = "test content".as_bytes().to_vec();

        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/objects");

        let local_storage = LocalStorage::init(source.clone());
        assert!(local_storage
            .put(&meta.oid, meta.size, &content)
            .await
            .is_ok());

        assert!(local_storage.exist(&meta.oid));
    }
}
