//!
//!
//!
//!
//!
//!

use std::fs;
use std::sync::{Arc, Mutex};
use std::{ops::Deref, path::PathBuf};

use venus::hash::SHA1;
use venus::internal::object::types::ObjectType;
use dashmap::{DashMap, DashSet};
use serde::{Deserialize, Serialize};
use crate::internal::pack::utils;
use lru_mem::{HeapSize, LruCache};

#[allow(unused)]
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// ! used by lru_mem to caculate the size of the object, limit the memory usage.
/// ! the implementation of HeapSize is not accurate, only caculate the size of the data_decompress
impl HeapSize for CacheObject {
    fn heap_size(&self) -> usize {
        self.data_decompress.heap_size()
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
    map_offset: DashMap<usize, SHA1>, // offset to hash
    hash_set: DashSet<SHA1>,          // item in the cache
    map_hash: Mutex<LruCache<String, ArcWrapper<CacheObject>>>, // !TODO: use SHA1 as key !TODO: interior mutability
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

/// !Implementing encapsulation of Arc<T> to enable third-party Trait HeapSize implementation for the Arc type
/// !Because of use Arc<T> in LruCache, the LruCache is not clear whether a pointer will drop the referenced
/// ! content when it is ejected from the cache, the actual memory usage is not accurate
struct ArcWrapper<T: HeapSize>(Arc<T>);
impl<T: HeapSize> HeapSize for ArcWrapper<T> {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}
impl<T: HeapSize> Clone for ArcWrapper<T> {
    fn clone(&self) -> Self {
        ArcWrapper(self.0.clone())
    }
}
impl<T: HeapSize> Deref for ArcWrapper<T> {
    type Target = Arc<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Caches {
    fn get_without_check(&self, hash: SHA1) -> Option<Arc<CacheObject>> {
        let rt = {
            let mut map = self.map_hash.lock().unwrap();
            match map.get(&hash.to_string()) {
                Some(x) => Ok(x.clone().0.clone()),
                None => Err("not found".to_string()),
            }
        };

        if rt.is_ok() {
            let obj = rt.unwrap();
            self.hash_set.insert(hash);
            return Some(obj);
        }

        // read from tmp file
        match self.read_from_tmp(hash) {
            Some(x) => {
                let mut map = self.map_hash.lock().unwrap();
                let x = ArcWrapper(Arc::new(x.clone()));
                let _ = map.insert(hash.to_string(), x.clone()); // handle the error
                Some(x.clone().0)
            }
            None => None, // not found, maybe trow some error
        }
    }
    fn generate_tmp_path(&self, hash: SHA1) -> PathBuf {
        let mut path = self.tmp_path.clone();
        path.push(hash.to_string());
        path
    }
    fn read_from_tmp(&self, hash: SHA1) -> Option<CacheObject> {
        let path = self.generate_tmp_path(hash);
        let b = fs::read(path).unwrap();
        let obj: CacheObject = bincode::deserialize(&b).unwrap();
        Some(obj)
    }

    /// ! write the object to tmp file, use another thread to do this latter
    fn write_to_tmp(&self, hash: SHA1, obj: &CacheObject) {
        let path = self.generate_tmp_path(hash);
        let b = bincode::serialize(&obj).unwrap();
        fs::write(path, b).unwrap();
    }
}

impl _Cache for Caches {
    fn new(size: Option<usize>, tmp_path: Option<PathBuf>) -> Self
    where
        Self: Sized,
    {
        Caches {
            map_offset: DashMap::new(),
            hash_set: DashSet::new(),
            map_hash: Mutex::new(LruCache::new(size.unwrap_or(0))),
            mem_size: size.unwrap_or(0),
            tmp_path: tmp_path.unwrap_or(PathBuf::from("tmp/")),
        }
    }
    fn get_hash(&self, offset: usize) -> Option<SHA1> {
        self.map_offset.get(&offset).map(|x| *x)
    }
    fn insert(&self, offset: usize, hash: SHA1, obj: CacheObject) {
        {
            // ? whether insert to cache directly or write to tmp file
            let mut map = self.map_hash.lock().unwrap();
            let _ = map.insert(hash.to_string(), ArcWrapper(Arc::new(obj.clone())));
            // handle the error
        }
        self.map_offset.insert(offset, hash);
        self.hash_set.insert(hash);
        self.write_to_tmp(hash, &obj);
    }
    fn get_by_offset(&self, offset: usize) -> Option<Arc<CacheObject>> {
        match self.map_offset.get(&offset) {
            Some(x) => self.get_by_hash(*x),
            None => None,
        }
    }
    fn get_by_hash(&self, hash: SHA1) -> Option<Arc<CacheObject>> {
        // check if the hash is in the cache( lru or tmp file)
        match self.get_without_check(hash) {
            Some(_) => self.get_without_check(hash),
            None => None,
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Mutex;

    use lru_mem::LruCache;

    use venus::{hash::SHA1, internal::object::types::ObjectType};

    use super::*;

    #[test]
    fn test_cache_object_with_same_size() {
        let a = CacheObject {
            base_offset: 0,
            base_ref: SHA1::new(&vec![0; 20]),
            data_decompress: vec![0; 1024],
            obj_type: ObjectType::Blob,
            offset: 0,
            hash: SHA1::new(&vec![0; 20]),
        };
        assert!(a.heap_size() == 1024);

        let b = ArcWrapper(Arc::new(a.clone()));
        assert!(b.heap_size() == 1024);
    }
    #[test]
    fn test_chache_object_with_lru() {
        let mut cach = LruCache::new(2048);
        let a = CacheObject {
            base_offset: 0,
            base_ref: SHA1::new(&vec![0; 20]),
            data_decompress: vec![0; 1024],
            obj_type: ObjectType::Blob,
            offset: 0,
            hash: SHA1::new(&vec![0; 20]),
        };
        println!("a.heap_size() = {}", a.heap_size());

        let b = CacheObject {
            base_offset: 0,
            base_ref: SHA1::new(&vec![0; 20]),
            data_decompress: vec![0; (1024.0 * 1.5) as usize],
            obj_type: ObjectType::Blob,
            offset: 0,
            hash: SHA1::new(&vec![1; 20]),
        };
        {
            let r = cach.insert(a.hash.to_string(), ArcWrapper(Arc::new(a.clone())));
            assert!(r.is_ok())
        }
        {
            let r = cach.try_insert(b.clone().hash.to_string(), ArcWrapper(Arc::new(b.clone())));
            assert!(r.is_err());
            if let Err(lru_mem::TryInsertError::WouldEjectLru { .. }) = r {
                // 匹配到指定错误，不需要额外操作
            } else {
                panic!("Expected WouldEjectLru error");
            }
            let r = cach.insert(b.hash.to_string(), ArcWrapper(Arc::new(b.clone())));
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
            let _ = c.insert("a", ArcWrapper(Arc::new(Test { a: 1024 })));
        }
        println!("insert b, a should be ejected");
        {
            let mut c = cach.as_ref().lock().unwrap();
            let _ = c.insert("b", ArcWrapper(Arc::new(Test { a: 1200 })));
        }
        let b = {
            let mut c = cach.as_ref().lock().unwrap();
            c.get("b").cloned()
        };
        println!("insert c, b should not be ejected");
        {
            let mut c = cach.as_ref().lock().unwrap();
            let _ = c.insert("c", ArcWrapper(Arc::new(Test { a: 1200 })));
        }
        println!("user b: {}", b.as_ref().unwrap().a);
        println!("test over, enject all");
    }

    #[test]
    fn test_cache_object_serialize() {
        let a = CacheObject {
            base_offset: 0,
            base_ref: SHA1::new(&vec![0; 20]),
            data_decompress: vec![0; 1024],
            obj_type: ObjectType::Blob,
            offset: 0,
            hash: SHA1::new(&vec![0; 20]),
        };
        let s = bincode::serialize(&a).unwrap();
        let b: CacheObject = bincode::deserialize(&s).unwrap();
        assert!(a.base_offset == b.base_offset);
    }

    // fn test
}
