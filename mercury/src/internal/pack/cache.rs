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
use lru_mem::HeapSize;

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

/// used by lru_mem to caculate the size of the object, limit the memory usage
impl HeapSize for CacheObject {
    fn heap_size(&self) -> usize {
        self.data_decompress.heap_size()
    }
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

impl Caches {}

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
mod test {
    use std::sync::Mutex;

    use lru_mem::{LruCache, ValueSize};

    use crate::{hash::SHA1, internal::object::types::ObjectType};

    use super::*;

    #[test]
    fn test_cache_object_with_same_size() {
        let a = CacheObject {
            base_offset: 0,
            base_ref: SHA1::new(&vec![0; 20]),
            data_decompress: vec![0; 1024],
            object_type: ObjectType::Blob,
            offset: 0,
            hash: SHA1::new(&vec![0; 20]),
        };
        assert!(a.heap_size() == 1024);

        let b = Arc::new(a);
        assert!(b.heap_size() == 1024);
    }
    #[test]
    fn test_chache_object_with_lru() {
        // let mut cach: LruCache<SHA1, Arc<CacheObject>> = LruCache::new(2048);
        let mut cach = LruCache::new(2048);
        let a = CacheObject {
            base_offset: 0,
            base_ref: SHA1::new(&vec![0; 20]),
            data_decompress: vec![0; 1024],
            object_type: ObjectType::Blob,
            offset: 0,
            hash: SHA1::new(&vec![0; 20]),
        };
        println!("a.value_size() = {}", a.value_size());
        println!("a.heap_size() = {}", a.heap_size());

        let b = CacheObject {
            base_offset: 0,
            base_ref: SHA1::new(&vec![0; 20]),
            data_decompress: vec![0; (1024.0 * 1.5) as usize],
            object_type: ObjectType::Blob,
            offset: 0,
            hash: SHA1::new(&vec![1; 20]),
        };
        {
            let r = cach.insert(a.hash.to_string(), Arc::new(a.clone()));
            assert!(r.is_ok())
        }
        {
            let r = cach.try_insert(b.clone().hash.to_string(), Arc::new(b.clone()));
            assert!(r.is_err());
            if let Err(lru_mem::TryInsertError::WouldEjectLru { .. }) = r {
                // 匹配到指定错误，不需要额外操作
            } else {
                panic!("Expected WouldEjectLru error");
            }
            let r = cach.insert(b.hash.to_string(), Arc::new(b.clone()));
            assert!(r.is_ok());
        }
        {
            // a should be ejected
            let r = cach.get(&a.hash.to_string());
            assert!(r.is_none());
        }
    }

    #[test]
    fn test_lru_drop() {
        struct Test {
            a: usize,
        }
        impl Drop for Test {
            fn drop(&mut self) {
                println!("drop Test");
            }
        }
        impl HeapSize for Test {
            fn heap_size(&self) -> usize {
                self.a
            }
        }
        println!("insert a");
        let cach = LruCache::new(2048);
        let cach = Arc::new(Mutex::new(cach));
        {
            let mut c = cach.as_ref().lock().unwrap();
            let _ = c.insert("a", Arc::new(Test { a: 1200 }));
        }
        println!("insert b, a should be ejected");
        {
            let mut c = cach.as_ref().lock().unwrap();
            let _ = c.insert("b", Arc::new(Test { a: 1200 }));
        }
        let b = {
            let mut c = cach.as_ref().lock().unwrap();
            c.get("b").cloned()
        };
        println!("insert c, b should not be ejected");
        {
            let mut c = cach.as_ref().lock().unwrap();
            let _ = c.insert("c", Arc::new(Test { a: 1200 }));
        }
        print!("user b: {}", b.as_ref().unwrap().a);
        println!("test over, enject all");
    }
}
