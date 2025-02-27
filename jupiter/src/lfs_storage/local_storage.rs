use std::fs::{self, OpenOptions};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;

use common::errors::MegaError;

use crate::lfs_storage::LfsStorage;

#[derive(Default)]
pub struct LocalStorage {
    base_path: PathBuf,
}

impl LocalStorage {
    pub fn init(base_path: PathBuf) -> LocalStorage {
        fs::create_dir_all(&base_path).expect("Create directory failed!");
        LocalStorage { base_path }
    }
}

#[async_trait]
impl LfsStorage for LocalStorage {
    async fn get_ref(&self, repo_id: i64, ref_name: &str) -> Result<String, MegaError> {
        let path = Path::new(&self.base_path)
            .join(repo_id.to_string())
            .join(ref_name);
        let mut file = fs::File::open(path)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        Ok(buf)
    }

    async fn put_ref(&self, repo_id: i64, ref_name: &str, ref_hash: &str) -> Result<(), MegaError> {
        let path = Path::new(&self.base_path)
            .join(repo_id.to_string())
            .join(ref_name);
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent)?;
        let mut file = fs::File::create(path)?;
        file.write_all(ref_hash.as_bytes())?;
        Ok(())
    }

    async fn delete_ref(&self, repo_id: i64, ref_name: &str) -> Result<(), MegaError> {
        let path = Path::new(&self.base_path)
            .join(repo_id.to_string())
            .join(ref_name);
        Ok(fs::remove_file(path)?)
    }

    async fn update_ref(
        &self,
        repo_id: i64,
        ref_name: &str,
        ref_hash: &str,
    ) -> Result<(), MegaError> {
        let path = Path::new(&self.base_path)
            .join(repo_id.to_string())
            .join(ref_name);
        let mut file = OpenOptions::new().write(true).open(path).unwrap();
        file.write_all(ref_hash.as_bytes()).unwrap();
        Ok(())
    }

    async fn get_object(&self, object_id: &str) -> Result<Bytes, MegaError> {
        let path = Path::new(&self.base_path)
            .join("objects")
            .join(self.transform_path(object_id));
        let mut file =
            fs::File::open(&path).unwrap_or_else(|_| panic!("Open file:{:?} failed!", path));
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        Ok(Bytes::from(buffer))
    }

    async fn put_object(&self, object_id: &str, body_content: &[u8]) -> Result<String, MegaError> {
        let path = Path::new(&self.base_path)
            .join("objects")
            .join(self.transform_path(object_id));
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir).expect("Create directory failed!");

        let mut file = fs::File::create(&path).expect("Open file failed");
        file.write_all(body_content).expect("Write file failed");
        Ok(path.to_str().unwrap().to_string())
    }

    fn exist_object(&self, object_id: &str) -> bool {
        let path = Path::new(&self.base_path)
            .join("objects")
            .join(self.transform_path(object_id));
        Path::exists(&path)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::{env, path::PathBuf};

    use crate::lfs_storage::{local_storage::LocalStorage, LfsStorage};

    #[tokio::test]
    async fn test_content_store() {
        let oid = "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72".to_owned();
        let content = "test content".as_bytes().to_vec();

        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/objects");

        let local_storage = LocalStorage::init(source.clone());
        assert!(local_storage.put_object(&oid, &content).await.is_ok());

        assert!(local_storage.exist_object(&oid));
    }

    #[tokio::test]
    async fn test_put_ref() {
        let test_path = PathBuf::from(env::current_dir().unwrap().parent().unwrap()).join("test");
        let storage = LocalStorage::init(test_path.clone());
        let ref_path = test_path.join("refs/tags/1.0");

        storage
            .put_ref(
                0,
                "refs/tags/1.0",
                "5bb8ee25bac1014c15abc49c56d1ee0aab1050cb",
            )
            .await
            .unwrap();

        assert!(Path::exists(&ref_path));
        fs::remove_file(ref_path).unwrap();
    }

    #[tokio::test]
    async fn test_update_ref() {
        let test_path = PathBuf::from(env::current_dir().unwrap().parent().unwrap()).join("tests");
        let storage = LocalStorage::init(test_path.clone());
        let ref_name = "refs/tags/2.0";
        let ref_path = test_path.join(ref_name);

        // init file
        fs::write(&ref_path, "aa33dc413d3845d631d57169d87020f5c61c8652").unwrap();

        // run test code
        storage
            .update_ref(0, ref_name, "04ea005354bbbf8bf676fd97d8993a66ffeaa472")
            .await
            .unwrap();
        let buf = fs::read_to_string(&ref_path).unwrap();
        assert_eq!(buf, "04ea005354bbbf8bf676fd97d8993a66ffeaa472");
        // clean up resources
        fs::remove_file(ref_path).unwrap();
    }

    #[tokio::test]
    async fn test_delete_ref() {
        let test_path = PathBuf::from(env::current_dir().unwrap().parent().unwrap()).join("tests");
        let ref_name = "refs/tags/3.0";
        let ref_path = test_path.join(ref_name);

        fs::write(&ref_path, "5bb8ee25bac1014c15abc49c56d1ee0aab1050cb").unwrap();

        let storage = LocalStorage::init(test_path.clone());
        storage.delete_ref(0, ref_name).await.unwrap();
        let ref_path = test_path.join(ref_name);
        assert!(!Path::exists(&ref_path));
    }

    #[tokio::test]
    async fn test_get_ref() {
        let mut test_path = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        test_path.push("tests");
        let storage = LocalStorage::init(test_path.clone());
        let ref_hash = storage.get_ref(0, "refs/heads/master").await.unwrap();
        assert_eq!(ref_hash, "5bb8ee25bac1014c15abc49c56d1ee0aab1050cb")
    }
}
