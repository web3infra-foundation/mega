//!
//!
//!
//!
//!
//!

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use venus::hash::SHA1;
use venus::internal::object::types::ObjectType;
use venus::internal::object::ObjectTrait;

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct CacheObject {
    pub base_offset: usize,
    pub base_ref: SHA1,
    pub data_decompress: Vec<u8>,
    pub object_type: ObjectType,
    pub offset: usize,
    pub hash: SHA1,
}

pub trait _Cache {
    fn new(size: Option<usize>) -> Self where Self: Sized;
    fn get_hash(&self, offset: usize) -> Option<SHA1>;
    fn get_by_offset(&self, offset: usize) -> Option<Arc<CacheObject>>;
    fn insert(&self, offset: usize, hash: SHA1, obj: CacheObject);
    fn get_by_hash(&self, h: SHA1) -> Option<Arc<CacheObject>>;
}

#[allow(unused)]
pub struct Caches {
    pub map_offset: HashMap<usize, SHA1>,
    pub map_hash: HashMap<SHA1, Arc<CacheObject>>,
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
        // self.map_offset.insert(offset, object);

        self.mem_size += self.get(offset).unwrap().data_decompress.len();
    }

    pub fn get(&self, offset: usize) -> Option<&CacheObject> {
        self.map_offset.get(&offset)
    }
}
