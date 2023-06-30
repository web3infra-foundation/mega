use sha256::digest;
use std::fs;
use std::io::prelude::*;
use std::path;
use std::path::PathBuf;

pub struct ContentStore {
    base_path: PathBuf,
}

#[derive(Debug, Default)]
pub struct MetaObject {
    pub oid: String,
    pub size: i64,
    pub exist: bool,
}

impl ContentStore {
    pub fn new(base: PathBuf) -> ContentStore {
        fs::create_dir_all(&base).expect("Create directory failed!");
        ContentStore { base_path: base }
    }

    pub fn get(&self, meta: &MetaObject, start: i64) -> fs::File {
        let path = path::Path::new(&self.base_path).join(transform_key(meta.oid.to_owned()));

        let mut file = fs::File::open(path).expect("Open file failed!");
        if start > 0 {
            file.seek(std::io::SeekFrom::Start(start as u64))
                .expect("Shift file pointer failed");
        }

        file
    }

    pub fn put(&self, meta: &MetaObject, body_content: &[u8]) -> bool {
        let path = path::Path::new(&self.base_path).join(transform_key(meta.oid.to_owned()));
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir).expect("Create directory failed!");

        let mut file = fs::File::create(&path).expect("Open file failed");
        let lenght_written = file.write(body_content).expect("Write file failed");
        if lenght_written as i64 != meta.size {
            return false;
        }

        let hash = digest(body_content);
        if hash != meta.oid {
            return false;
        }
        true
    }

    pub fn exist(&self, meta: &MetaObject) -> bool {
        let path = path::Path::new(&self.base_path).join(transform_key(meta.oid.to_owned()));

        path::Path::exists(&path)
    }
}

fn transform_key(key: String) -> String {
    if key.len() < 5 {
        key
    } else {
        path::Path::new(&key[0..2])
            .join(&key[2..4])
            .join(&key[4..key.len()])
            .into_os_string()
            .into_string()
            .unwrap()
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
        source.push("tests/data/objects");

        let content_store = ContentStore::new(source);
        assert!(content_store.put(&meta, content));

        assert!(content_store.exist(&meta));
    }
}
