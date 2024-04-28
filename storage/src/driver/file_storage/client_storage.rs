use std::fs;
use std::io::prelude::*;
use std::path;
use std::path::PathBuf;

use async_trait::async_trait;
use bytes::Bytes;

use common::errors::MegaError;

use crate::driver::file_storage::FileStorage;

#[derive(Default)]
pub struct ClientStorage {
    base_path: PathBuf,
}

impl ClientStorage {
    /// create `base_path` directory
    /// - `base_path` should be ".../objects"
    pub fn init(base_path: PathBuf) -> ClientStorage {
        fs::create_dir_all(&base_path).expect("Create directory failed!");
        ClientStorage { base_path }
    }

    /// e.g. 6ae8a755... -> 6a/e8a755...
    fn transform_path(&self, path: &str) -> String {
        path::Path::new(&path[0..2])
            .join(&path[2..path.len()])
            .into_os_string()
            .into_string()
            .unwrap()
    }

    pub fn search(&self, obj_id: &str) -> Vec<String> {
        self.list_objects()
            .into_iter()
            .filter(|x| x.starts_with(obj_id))
            .collect()
    }

    pub fn list_objects(&self) -> Vec<String> {
        let mut objects = Vec::new();
        let paths = fs::read_dir(&self.base_path).expect("Read directory failed!");
        for path in paths {
            let path = path.unwrap().path();
            if path.is_dir() && path.file_name().unwrap().len() == 2 {
                let sub_paths = fs::read_dir(&path).expect("Read directory failed!");
                for sub_path in sub_paths {
                    let sub_path = sub_path.unwrap().path();
                    if sub_path.is_file() {
                        let parent_name = path.file_name().unwrap().to_str().unwrap().to_string();
                        let file_name = sub_path.file_name().unwrap().to_str().unwrap().to_string();
                        let file_name = parent_name + &file_name;
                        objects.push(file_name);
                    }
                }
            }
        }
        objects
    }
}

#[async_trait]
impl FileStorage for ClientStorage {
    async fn get(&self, object_id: &str) -> Result<Bytes, MegaError> {
        let path = path::Path::new(&self.base_path).join(self.transform_path(object_id));
        let mut file = fs::File::open(&path).unwrap_or_else(|_| panic!("Open file:{:?} failed!", path));
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        Ok(Bytes::from(buffer))
    }

    /// Save content to `objects`
    /// - `_size` is ignored
    async fn put(
        &self,
        object_id: &str,
        _ignore_size: i64,
        body_content: &[u8],
    ) -> Result<String, MegaError> {
        let path = path::Path::new(&self.base_path).join(self.transform_path(object_id));
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir).expect("Create directory failed!");

        let mut file = fs::File::create(&path).expect("Open file failed");
        file.write_all(body_content).expect("Write file failed");
        // TODO LocalStorage::put 使用了`write` 而不是 `write_all`，可能会导致写入不完整
        Ok(path.to_str().unwrap().to_string())
    }

    fn exist(&self, object_id: &str) -> bool {
        let path = path::Path::new(&self.base_path).join(self.transform_path(object_id));
        path::Path::exists(&path)
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::PathBuf;
    use crate::driver::file_storage::client_storage::ClientStorage;
    use crate::driver::file_storage::FileStorage;

    #[derive(Debug, Default)]
    pub struct MetaObject {
        pub oid: String,
        pub size: i64,
        pub exist: bool,
    }

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

        let client_storage = ClientStorage::init(source.clone());
        assert!(client_storage
            .put(&meta.oid, meta.size, &content)
            .await
            .is_ok());

        assert!(client_storage.exist(&meta.oid));
    }

    #[tokio::test]
    async fn test_search() {
        let meta = MetaObject {
            oid: "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72".to_owned(),
            size: 12,
            exist: false,
        };

        let content = "test content".as_bytes().to_vec();

        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/objects");

        let client_storage = ClientStorage::init(source.clone());
        assert!(client_storage
            .put(&meta.oid, meta.size, &content)
            .await
            .is_ok());

        let objs = client_storage.search("6ae8a755");
        println!("{:?}", objs);

        assert_eq!(objs.len(), 1);
    }
}
