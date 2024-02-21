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
use dashmap::DashMap;

use crate::internal::pack::utils;

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct CacheObject {
    pub base_offset: usize,
    pub base_ref: SHA1,
    pub data_decompress: Vec<u8>,
    pub obj_type: ObjectType,
    pub offset: usize,
    pub hash: SHA1,
}
// For Convenience
impl Default for CacheObject {
    fn default() -> Self {
        CacheObject {
            base_offset: 0,
            base_ref: SHA1::default(),
            data_decompress: Vec::new(),
            obj_type: ObjectType::Blob,
            offset: 0,
            hash: SHA1::default(),
        }
    }
}

pub trait _Cache {
    fn new(size: Option<usize>, tmp_path: Option<PathBuf>) -> Self
    where
        Self: Sized;
    fn get_hash(&self, offset: usize) -> Option<SHA1>;
    fn insert(&self, offset: usize, hash: SHA1, obj: CacheObject);
    fn get_by_offset(&self, offset: usize) -> Option<Arc<CacheObject>>;
    fn get_by_hash(&self, h: SHA1) -> Option<Arc<CacheObject>>;
}

#[allow(unused)]
pub struct Caches {
    map_offset: DashMap<usize, SHA1>,
    map_hash: HashMap<SHA1, Arc<CacheObject>>,
    mem_size: usize,
    tmp_path: PathBuf,
}

impl CacheObject {
    pub fn new_for_undeltified(obj_type: ObjectType, data: Vec<u8>, offset: usize) -> Self {
        let hash = utils::calculate_object_hash(obj_type, &data);
        CacheObject {
            data_decompress: data,
            obj_type,
            offset,
            hash,
            ..Default::default()
        }

    }
}

impl Caches {
    
}

impl _Cache for Caches {
    fn new(size: Option<usize>, tmp_path: Option<PathBuf>) -> Self
    where
        Self: Sized,
    {
        Caches {
            map_offset: DashMap::new(),
            map_hash: HashMap::new(),
            mem_size: size.unwrap_or(0),
            tmp_path: tmp_path.unwrap_or_default(),
        }
    }
    fn get_hash(&self, offset: usize) -> Option<SHA1> {
        self.map_offset.get(&offset).map(|x| *x)
    }
    fn insert(&self, offset: usize, hash: SHA1, obj: CacheObject) {
        self.map_offset.insert(offset, hash);
        unimplemented!()
    }
    fn get_by_offset(&self, offset: usize) -> Option<Arc<CacheObject>> {
        match self.map_offset.get(&offset) {
            Some(x) => self.get_by_hash(*x),
            None => None,
        }
    }
    fn get_by_hash(&self, hash: SHA1) -> Option<Arc<CacheObject>> {
        unimplemented!()
    }
}


#[cfg(test)]
mod test{

}