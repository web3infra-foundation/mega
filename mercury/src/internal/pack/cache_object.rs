use std::{ops::Deref, sync::Arc};

use crate::internal::pack::utils;
use lru_mem::HeapSize;
use serde::{Deserialize, Serialize};
use venus::{hash::SHA1, internal::object::types::ObjectType};

#[allow(unused)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheObject {
    pub base_offset: usize,
    pub base_ref: SHA1,
    pub obj_type: ObjectType,
    pub data_decompress: Vec<u8>,
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

// ! used by lru_mem to calculate the size of the object, limit the memory usage.
// ! the implementation of HeapSize is not accurate, only calculate the size of the data_decompress
impl HeapSize for CacheObject {
    fn heap_size(&self) -> usize {
        self.data_decompress.heap_size()
    }
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

    pub fn to_entry(&self) -> venus::internal::pack::entry::Entry {
        match self.obj_type {
            ObjectType::Blob | ObjectType::Tree | ObjectType::Commit | ObjectType::Tag => {
                venus::internal::pack::entry::Entry {
                    header: venus::internal::pack::header::EntryHeader::from_string(
                        self.obj_type.to_string().as_str(),
                    ),
                    offset: self.offset,
                    data: self.data_decompress.clone(),
                    hash: Some(self.hash),
                }
            }
            ObjectType::OffsetDelta => {
                venus::internal::pack::entry::Entry {
                    header: venus::internal::pack::header::EntryHeader::OfsDelta {
                        base_distance: self.offset - self.base_offset, // ?  is the distance is what we want?
                    },
                    offset: self.offset,
                    data: self.data_decompress.clone(),
                    hash: Some(self.hash),
                }
            }
            ObjectType::HashDelta => venus::internal::pack::entry::Entry {
                header: venus::internal::pack::header::EntryHeader::RefDelta {
                    base_id: self.base_ref,
                },
                offset: self.offset,
                data: self.data_decompress.clone(),
                hash: Some(self.hash),
            },
        }
    }
}

/// !Implementing encapsulation of Arc<T> to enable third-party Trait HeapSize implementation for the Arc type
/// !Because of use Arc<T> in LruCache, the LruCache is not clear whether a pointer will drop the referenced
/// ! content when it is ejected from the cache, the actual memory usage is not accurate
pub struct ArcWrapper<T: HeapSize>(pub Arc<T>);
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

#[cfg(test)]
mod test {
    use std::sync::Mutex;

    use lru_mem::LruCache;

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
        let mut cache = LruCache::new(2048);
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
            let r = cache.insert(a.hash.to_plain_str(), ArcWrapper(Arc::new(a.clone())));
            assert!(r.is_ok())
        }
        {
            let r = cache.try_insert(
                b.clone().hash.to_plain_str(),
                ArcWrapper(Arc::new(b.clone())),
            );
            assert!(r.is_err());
            if let Err(lru_mem::TryInsertError::WouldEjectLru { .. }) = r {
                // 匹配到指定错误，不需要额外操作
            } else {
                panic!("Expected WouldEjectLru error");
            }
            let r = cache.insert(b.hash.to_plain_str(), ArcWrapper(Arc::new(b.clone())));
            assert!(r.is_ok());
        }
        {
            // a should be ejected
            let r = cache.get(&a.hash.to_plain_str());
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
        let cache = LruCache::new(2048);
        let cache = Arc::new(Mutex::new(cache));
        {
            let mut c = cache.as_ref().lock().unwrap();
            let _ = c.insert("a", ArcWrapper(Arc::new(Test { a: 1024 })));
        }
        println!("insert b, a should be ejected");
        {
            let mut c = cache.as_ref().lock().unwrap();
            let _ = c.insert("b", ArcWrapper(Arc::new(Test { a: 1200 })));
        }
        let b = {
            let mut c = cache.as_ref().lock().unwrap();
            c.get("b").cloned()
        };
        println!("insert c, b should not be ejected");
        {
            let mut c = cache.as_ref().lock().unwrap();
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
}
