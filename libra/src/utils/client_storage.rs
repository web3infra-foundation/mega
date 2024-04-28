use std::{fs, io};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use venus::errors::GitError;

use venus::hash::SHA1;
use venus::internal::object::types::ObjectType;

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
    fn transform_path(&self, hash: &SHA1) -> String {
        let hash = hash.to_plain_str();
        Path::new(&hash[0..2])
            .join(&hash[2..hash.len()])
            .into_os_string()
            .into_string()
            .unwrap()
    }

    pub fn get_object_type(&self, obj_id: &SHA1) -> Result<ObjectType, GitError> {
        let raw_data = self.read_raw_data(obj_id)?;
        let data = Self::decompress_zlib(&raw_data)?;
        let (obj_type, _, _) = Self::parse_header(&data);
        ObjectType::from_string(&obj_type)
    }

    /// Search objects that start with `obj_id`
    pub fn search(&self, obj_id: &str) -> Vec<SHA1> {
        self.list_objects()
            .into_iter()
            .filter(|x| x.to_plain_str().starts_with(obj_id))
            .collect()
    }

    /// list all objects' hash in `objects`
    pub fn list_objects(&self) -> Vec<SHA1> {
        let mut objects = Vec::new();
        let paths = fs::read_dir(&self.base_path).unwrap();
        for path in paths {
            let path = path.unwrap().path();
            if path.is_dir() && path.file_name().unwrap().len() == 2 { // not very elegant
                let sub_paths = fs::read_dir(&path).unwrap();
                for sub_path in sub_paths {
                    let sub_path = sub_path.unwrap().path();
                    if sub_path.is_file() {
                        let parent_name = path.file_name().unwrap().to_str().unwrap().to_string();
                        let file_name = sub_path.file_name().unwrap().to_str().unwrap().to_string();
                        let file_name = parent_name + &file_name;
                        objects.push(SHA1::from_str(&file_name).unwrap()); // this will check format, so don't worry
                    }
                }
            }
        }
        objects
    }
}

impl ClientStorage {
    /// zlib header: 78 9C, but Git is 78 01
    fn compress_zlib(data: &[u8]) -> io::Result<Vec<u8>> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        let compressed_data = encoder.finish()?;
        Ok(compressed_data)
    }

    fn decompress_zlib(data: &[u8]) -> io::Result<Vec<u8>> {
        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;
        Ok(decompressed_data)
    }

    fn parse_header(data: &[u8]) -> (String, usize, usize) {
        let end_of_header = data.iter()
            .position(|&b| b == b'\0')
            .expect("Invalid object: no header terminator");
        let header_str = std::str::from_utf8(&data[..end_of_header])
            .expect("Invalid UTF-8 in header");

        let mut parts = header_str.splitn(2, ' ');
        let obj_type = parts.next().expect("No object type in header").to_string();
        let size_str = parts.next().expect("No size in header");
        let size = size_str.parse::<usize>().expect("Invalid size in header");
        assert_eq!(size, data.len() - 1 - end_of_header, "Invalid object size");
        (obj_type, size, end_of_header)
    }

    fn read_raw_data(&self, object_id: &SHA1) -> Result<Vec<u8>, io::Error> {
        let path = Path::new(&self.base_path).join(self.transform_path(object_id));
        let mut file = fs::File::open(&path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    pub fn get(&self, object_id: &SHA1) -> Result<Vec<u8>, io::Error> {
        let raw_data = self.read_raw_data(object_id)?;
        let data = Self::decompress_zlib(&raw_data)?;

        // skip & check header
        let (_, _, end_of_header) = Self::parse_header(&data);
        Ok(data[end_of_header + 1..].to_vec())
    }

    /// Save content to `objects`
    pub fn put(&self, obj_id: &SHA1, content: &[u8], obj_type: ObjectType) -> Result<String, io::Error> {
        let path = Path::new(&self.base_path).join(self.transform_path(obj_id));
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir)?;

        let header = format!("{} {}\0", obj_type.to_string(), content.len());
        let full_content = [header.as_bytes().to_vec(), Vec::from(content)].concat();

        let mut file = fs::File::create(&path)?;
        file.write_all(&Self::compress_zlib(&full_content)?)?;
        Ok(path.to_str().unwrap().to_string())
    }

    pub fn exist(&self, obj_id: &SHA1) -> bool {
        let path = Path::new(&self.base_path).join(self.transform_path(obj_id));
        Path::exists(&path)
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::PathBuf;

    use venus::internal::object::blob::Blob;
    use venus::internal::object::ObjectTrait;
    use venus::internal::object::types::ObjectType;

    use crate::utils::{test, util};

    use super::ClientStorage;

    #[test]
    fn test_content_store() {
        let content = "Hello, world!";
        let blob = Blob::from_content(content);

        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/objects");

        let client_storage = ClientStorage::init(source.clone());
        assert!(client_storage.put(&blob.id, &blob.data, blob.get_type()).is_ok());
        assert!(client_storage.exist(&blob.id));

        let data = client_storage.get(&blob.id).unwrap();
        assert_eq!(data, blob.data);
        assert_eq!(String::from_utf8(data).unwrap(), content);
    }

    #[test]
    fn test_search() {
        let blob = Blob::from_content("Hello, world!");

        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/objects");

        let client_storage = ClientStorage::init(source.clone());
        assert!(client_storage.put(&blob.id, &blob.data, blob.get_type()).is_ok());

        let objs = client_storage.search("5dd01c177");

        assert_eq!(objs.len(), 1);
    }

    #[test]
    fn test_list_objs() {
        let source = PathBuf::from(test::TEST_DIR).join(util::ROOT_DIR).join("objects");
        if !source.exists() {
            return;
        }
        let client_storage = ClientStorage::init(source);
        let objs = client_storage.list_objects();
        for obj in objs {
            println!("{}", obj);
        }
    }

    #[test]
    fn test_get_obj_type() {
        let blob = Blob::from_content("Hello, world!");

        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/objects");

        let client_storage = ClientStorage::init(source.clone());
        assert!(client_storage.put(&blob.id, &blob.data, blob.get_type()).is_ok());

        let obj_type = client_storage.get_object_type(&blob.id).unwrap();
        assert_eq!(obj_type, ObjectType::Blob);
    }

    #[test]
    fn test_decompress() {
        let data = b"blob 13\0Hello, world!";
        let compressed_data = ClientStorage::compress_zlib(data).unwrap();
        let decompressed_data = ClientStorage::decompress_zlib(&compressed_data).unwrap();
        assert_eq!(decompressed_data, data);
    }
}
