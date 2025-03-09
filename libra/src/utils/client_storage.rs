use std::collections::HashSet;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::{fs, io};

use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use lru_mem::LruCache;
use mercury::errors::GitError;
use mercury::hash::SHA1;
use mercury::internal::object::types::ObjectType;
use mercury::internal::pack::cache_object::CacheObject;
use mercury::internal::pack::Pack;
use mercury::utils::read_sha1;
use once_cell::sync::Lazy;

use crate::command;
static PACK_OBJ_CACHE: Lazy<Mutex<LruCache<String, CacheObject>>> = Lazy::new(|| {
    // `lazy_static!` may affect IDE's code completion
    Mutex::new(LruCache::new(1024 * 1024 * 200))
});

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
        let hash = hash.to_string();
        Path::new(&hash[0..2])
            .join(&hash[2..hash.len()])
            .into_os_string()
            .into_string()
            .unwrap()
    }

    /// join `base_path` and `obj_id` to get the full path of the object
    fn get_obj_path(&self, obj_id: &SHA1) -> PathBuf {
        Path::new(&self.base_path).join(self.transform_path(obj_id))
    }

    pub fn get_object_type(&self, obj_id: &SHA1) -> Result<ObjectType, GitError> {
        if self.exist_loosely(obj_id) {
            let raw_data = self.read_raw_data(obj_id)?;
            let data = Self::decompress_zlib(&raw_data)?;
            let (obj_type, _, _) = Self::parse_header(&data);
            ObjectType::from_string(&obj_type)
        } else {
            self.get_from_pack(obj_id)?
                .map(|x| x.1)
                .ok_or(GitError::ObjectNotFound(obj_id.to_string()))
        }
    }

    /// Check if the object with `obj_id` is of type `obj_type`
    pub fn is_object_type(&self, obj_id: &SHA1, obj_type: ObjectType) -> bool {
        match self.get_object_type(obj_id) {
            Ok(t) => t == obj_type,
            Err(_) => false,
        }
    }

    /// Search objects that start with `obj_id`, loose & pack
    pub fn search(&self, obj_id: &str) -> Vec<SHA1> {
        let mut objs = self.list_objects_pack();
        objs.extend(self.list_objects_loose());

        objs.into_iter()
            .filter(|x| x.to_string().starts_with(obj_id))
            .collect()
    }

    /// list all objects' hash in `objects`
    fn list_objects_loose(&self) -> Vec<SHA1> {
        let mut objects = Vec::new();
        let paths = fs::read_dir(&self.base_path).unwrap();
        for path in paths {
            let path = path.unwrap().path();
            if path.is_dir() && path.file_name().unwrap().len() == 2 {
                // not very elegant
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

    /// List all objects' hash in PACKs
    fn list_objects_pack(&self) -> HashSet<SHA1> {
        let idxes = self.list_all_idx();
        let mut objs = HashSet::new();
        for idx in idxes {
            let res = Self::list_idx_objects(&idx).unwrap();
            for obj in res {
                objs.insert(obj);
            }
        }
        objs
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
        let end_of_header = data
            .iter()
            .position(|&b| b == b'\0')
            .expect("Invalid object: no header terminator");
        let header_str =
            std::str::from_utf8(&data[..end_of_header]).expect("Invalid UTF-8 in header");

        let mut parts = header_str.splitn(2, ' ');
        let obj_type = parts.next().expect("No object type in header").to_string();
        let size_str = parts.next().expect("No size in header");
        let size = size_str.parse::<usize>().expect("Invalid size in header");
        assert_eq!(size, data.len() - 1 - end_of_header, "Invalid object size");
        (obj_type, size, end_of_header)
    }

    fn read_raw_data(&self, obj_id: &SHA1) -> Result<Vec<u8>, io::Error> {
        let path = self.get_obj_path(obj_id);
        let mut file = fs::File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    pub fn get(&self, object_id: &SHA1) -> Result<Vec<u8>, GitError> {
        if self.exist_loosely(object_id) {
            let raw_data = self.read_raw_data(object_id)?;
            let data = Self::decompress_zlib(&raw_data)?;

            // skip & check header
            let (_, _, end_of_header) = Self::parse_header(&data);
            Ok(data[end_of_header + 1..].to_vec())
        } else {
            // Ok(self.get_from_pack(object_id)?.unwrap().0)
            self.get_from_pack(object_id)?
                .map(|x| x.0)
                .ok_or(GitError::ObjectNotFound(object_id.to_string()))
        }
    }

    /// Save content to `objects`
    pub fn put(
        &self,
        obj_id: &SHA1,
        content: &[u8],
        obj_type: ObjectType,
    ) -> Result<String, io::Error> {
        let path = self.get_obj_path(obj_id);
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir)?;

        let header = format!("{} {}\0", obj_type, content.len());
        let full_content = [header.as_bytes().to_vec(), Vec::from(content)].concat();

        let mut file = fs::File::create(&path)?;
        file.write_all(&Self::compress_zlib(&full_content)?)?;
        Ok(path.to_str().unwrap().to_string())
    }

    /// Check if the object with `obj_id` exists in `objects` or PACKs
    pub fn exist(&self, obj_id: &SHA1) -> bool {
        let path = self.get_obj_path(obj_id);
        Path::exists(&path) || self.get_from_pack(obj_id).unwrap().is_some()
    }

    /// Check if the object with `obj_id` exists in `objects`
    fn exist_loosely(&self, obj_id: &SHA1) -> bool {
        let path = self.get_obj_path(obj_id);
        Path::exists(&path)
    }
}
const FANOUT: u64 = 256 * 4;
// TODO refactor to `PackReader`
impl ClientStorage {
    /// List all .pack files in `pack` directory
    fn list_all_packs(&self) -> Vec<PathBuf> {
        let pack_dir = self.base_path.join("pack");
        if !pack_dir.exists() {
            return Vec::new();
        }
        let mut packs = Vec::new();
        for entry in fs::read_dir(pack_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() && path.extension().unwrap() == "pack" {
                packs.push(path);
            }
        }
        packs
    }

    /// List all .idx files in `pack` directory
    /// - If .idx file not exists, build it
    fn list_all_idx(&self) -> Vec<PathBuf> {
        let packs = self.list_all_packs();
        let mut idxs = Vec::new();
        for pack in packs {
            let idx = pack.with_extension("idx");
            if !idx.exists() {
                command::index_pack::build_index_v1(pack.to_str().unwrap(), idx.to_str().unwrap())
                    .unwrap();
            }
            idxs.push(idx);
        }
        idxs
    }

    /// Get object from PACKs by hash, if not found, return None
    fn get_from_pack(&self, obj_id: &SHA1) -> Result<Option<(Vec<u8>, ObjectType)>, GitError> {
        let idxes = self.list_all_idx(); // list or build
        for idx in idxes {
            let res = Self::read_pack_by_idx(&idx, obj_id)?;
            if let Some(data) = res {
                return Ok(Some((data.data_decompressed.clone(), data.object_type())));
            }
        }

        Ok(None)
    }

    fn read_idx_fanout(idx_file: &Path) -> Result<[u32; 256], io::Error> {
        let mut idx_file = fs::File::open(idx_file)?;
        // const FANOUT: usize = 256 * 4;
        let mut fanout: [u32; 256] = [0; 256]; // 256 * 4 bytes
        let mut buf = [0; 4];
        fanout.iter_mut().for_each(|x| {
            idx_file.read_exact(&mut buf).unwrap();
            *x = u32::from_be_bytes(buf);
        });
        Ok(fanout)
    }

    /// List all objects hash in .idx file
    fn list_idx_objects(idx_file: &Path) -> Result<Vec<SHA1>, io::Error> {
        let fanout: [u32; 256] = Self::read_idx_fanout(idx_file)?; // TODO param change to `&mut File`, to auto seek
        let mut idx_file = fs::File::open(idx_file)?;
        idx_file.seek(io::SeekFrom::Start(FANOUT))?; // important!

        let mut objs = Vec::new();
        for _ in 0..fanout[255] {
            let _offset = idx_file.read_u32::<BigEndian>()?;
            let hash = read_sha1(&mut idx_file)?;

            objs.push(hash);
        }
        Ok(objs)
    }

    /// Read object `offset` from .idx file by `hash`
    fn read_idx(idx_file: &Path, obj_id: &SHA1) -> Result<Option<u64>, io::Error> {
        let fanout: [u32; 256] = Self::read_idx_fanout(idx_file)?;
        let mut idx_file = fs::File::open(idx_file)?;

        let first_byte = obj_id.0[0];
        let start = if first_byte == 0 {
            0
        } else {
            fanout[first_byte as usize - 1] as usize
        };
        let end = fanout[first_byte as usize] as usize;

        idx_file.seek(io::SeekFrom::Start(FANOUT + 24 * start as u64))?;
        for _ in start..end {
            let offset = idx_file.read_u32::<BigEndian>()?;
            let hash = read_sha1(&mut idx_file)?;

            if &hash == obj_id {
                return Ok(Some(offset as u64));
            }
        }

        Ok(None)
    }

    /// Get object from pack by .idx file
    fn read_pack_by_idx(idx_file: &Path, obj_id: &SHA1) -> Result<Option<CacheObject>, GitError> {
        let pack_file = idx_file.with_extension("pack");
        let res = Self::read_idx(idx_file, obj_id)?;
        match res {
            None => Ok(None),
            Some(offset) => {
                let res = Self::read_pack_obj(&pack_file, offset)?;
                Ok(Some(res))
            }
        }
    }

    /// Read object from pack file, with offset
    fn read_pack_obj(pack_file: &Path, offset: u64) -> Result<CacheObject, GitError> {
        let cache_key = format!("{:?}-{}", pack_file.file_name().unwrap(), offset);
        // read cache
        if let Some(cached) = PACK_OBJ_CACHE.lock().unwrap().get(&cache_key) {
            return Ok(cached.clone());
        }

        let file = fs::File::open(pack_file)?;
        let mut pack_reader = io::BufReader::new(&file);
        pack_reader.seek(io::SeekFrom::Start(offset))?;
        let mut pack = Pack::new(None, None, None, false);
        let obj = {
            let mut offset = offset as usize;
            pack.decode_pack_object(&mut pack_reader, &mut offset)? // offset will be updated!
        };
        let full_obj = match obj.object_type() {
            ObjectType::OffsetDelta => {
                let base_offset = obj.offset_delta().unwrap();
                let base_obj = Self::read_pack_obj(pack_file, base_offset as u64)?;
                let base_obj = Arc::new(base_obj);
                Pack::rebuild_delta(obj, base_obj) // new obj
            }
            ObjectType::HashDelta => {
                let base_hash = obj.hash_delta().unwrap();
                let idx_file = pack_file.with_extension("idx");
                let base_offset = Self::read_idx(&idx_file, &base_hash)?.unwrap();
                let base_obj = Self::read_pack_obj(pack_file, base_offset)?;
                let base_obj = Arc::new(base_obj);
                Pack::rebuild_delta(obj, base_obj) // new obj
            }
            _ => obj,
        };
        // write cache
        if PACK_OBJ_CACHE
            .lock()
            .unwrap()
            .insert(cache_key, full_obj.clone())
            .is_err()
        {
            eprintln!("Warn: EntryTooLarge");
        }
        Ok(full_obj)
    }
}

#[cfg(test)]
mod tests {
    use mercury::internal::object::blob::Blob;
    use mercury::internal::object::types::ObjectType;
    use mercury::internal::object::ObjectTrait;
    use serial_test::serial;
    use std::fs;
    use std::path::PathBuf;

    use crate::utils::{test, util};

    use super::ClientStorage;

    #[test]
    fn test_content_store() {
        let content = "Hello, world!";
        let blob = Blob::from_content(content);

        let mut source = PathBuf::from(test::find_cargo_dir().parent().unwrap());
        source.push("tests/objects");

        let client_storage = ClientStorage::init(source.clone());
        assert!(client_storage
            .put(&blob.id, &blob.data, blob.get_type())
            .is_ok());
        assert!(client_storage.exist(&blob.id));

        let data = client_storage.get(&blob.id).unwrap();
        assert_eq!(data, blob.data);
        assert_eq!(String::from_utf8(data).unwrap(), content);
    }

    #[test]
    fn test_search() {
        let blob = Blob::from_content("Hello, world!");

        let mut source = PathBuf::from(test::find_cargo_dir().parent().unwrap());
        source.push("tests/objects");

        let client_storage = ClientStorage::init(source.clone());
        assert!(client_storage
            .put(&blob.id, &blob.data, blob.get_type())
            .is_ok());

        let objs = client_storage.search("5dd01c177");

        assert_eq!(objs.len(), 1);
    }

    #[test]
    #[serial]
    fn test_list_objs() {
        test::reset_working_dir();
        let source = PathBuf::from(test::TEST_DIR)
            .join(util::ROOT_DIR)
            .join("objects");
        if !source.exists() {
            return;
        }
        let client_storage = ClientStorage::init(source);
        let objs = client_storage.list_objects_loose();
        for obj in objs {
            println!("{}", obj);
        }
    }

    #[test]
    fn test_get_obj_type() {
        let blob = Blob::from_content("Hello, world!");

        let mut source = PathBuf::from(test::find_cargo_dir().parent().unwrap());
        source.push("tests/objects");

        let client_storage = ClientStorage::init(source.clone());
        assert!(client_storage
            .put(&blob.id, &blob.data, blob.get_type())
            .is_ok());

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

    #[test]
    #[serial]
    fn test_decompress_2() {
        test::reset_working_dir();
        let pack_file = "../tests/data/objects/4b/00093bee9b3ef5afc5f8e3645dc39cfa2f49aa";
        let pack_content = fs::read(pack_file).unwrap();
        let decompressed_data = ClientStorage::decompress_zlib(&pack_content).unwrap();
        println!("{:?}", String::from_utf8(decompressed_data).unwrap());
    }

    #[test]
    #[ignore]
    fn test_get_from_pack() {
        unimplemented!();
    }
}
