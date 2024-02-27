//!
//!
//!
//!
//!
//!

use std::collections::HashMap;
use std::path::PathBuf;

use venus::hash::SHA1;
use venus::internal::object::types::ObjectType;
use venus::internal::object::ObjectTrait;

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct CacheObject {
    pub delta_offset: usize,
    pub delta_ref: SHA1,
    pub data_decompress: Vec<u8>,
    pub object_type: ObjectType,
    pub offset: usize,
}

#[allow(unused)]
pub struct Caches {
    pub objects: Vec<Box<dyn ObjectTrait>>,
    pub map_offset: HashMap<usize, CacheObject>,
    pub mem_size: usize,
    pub tmp_path: PathBuf,
}

impl CacheObject {}

impl Caches {
    ///
    ///
    ///
    ///
    pub fn insert(&mut self, offset: usize, object: CacheObject) {
        self.map_offset.insert(offset, object);

        self.mem_size += self.get(offset).unwrap().data_decompress.len();
    }

    pub fn get(&self, offset: usize) -> Option<&CacheObject> {
        self.map_offset.get(&offset)
    }
}
