//!
//! 
//! 
//! 
//! 
//! 

use std::collections::HashMap;

use crate::hash::SHA1;
use crate::internal::object::types::ObjectType;
use crate::internal::object::ObjectTrait;

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
}


impl CacheObject {
    
}


impl Caches {

    ///
    /// 
    /// 
    /// 
    pub fn insert(&mut self, offset: usize, object: CacheObject) {
        self.map_offset.insert(offset, object);
    }
}