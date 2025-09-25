use std::path::Path;
use std::path::PathBuf;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::{fs, io};

use dashmap::{DashMap, DashSet};
use lru_mem::LruCache;
use threadpool::ThreadPool;

use crate::hash::SHA1;
use crate::internal::pack::cache_object::{
    ArcWrapper, CacheObject, FileLoadStore, MemSizeRecorder,
};
use crate::time_it;

pub trait _Cache {
    fn new(mem_size: Option<usize>, tmp_path: PathBuf, thread_num: usize) -> Self
    where
        Self: Sized;
    fn get_hash(&self, offset: usize) -> Option<SHA1>;
    fn insert(&self, offset: usize, hash: SHA1, obj: CacheObject) -> Arc<CacheObject>;
    fn get_by_offset(&self, offset: usize) -> Option<Arc<CacheObject>>;
    fn get_by_hash(&self, h: SHA1) -> Option<Arc<CacheObject>>;
    fn total_inserted(&self) -> usize;
    fn memory_used(&self) -> usize;
    fn clear(&self);
}

impl lru_mem::HeapSize for SHA1 {
    fn heap_size(&self) -> usize {
        0
    }
}

pub struct Caches {
    map_offset: DashMap<usize, SHA1>, // offset to hash
    hash_set: DashSet<SHA1>,          // item in the cache
    // dropping large lru cache will take a long time on Windows without multi-thread IO
    // because "multi-thread IO" clone Arc<CacheObject>, so it won't be dropped in the main thread,
    // and `CacheObjects` will be killed by OS after Process ends abnormally
    // Solution: use `mimalloc`
    lru_cache: Mutex<LruCache<SHA1, ArcWrapper<CacheObject>>>,
    mem_size: Option<usize>,
    tmp_path: PathBuf,
    path_prefixes: [Once; 256],
    pool: Arc<ThreadPool>,
    complete_signal: Arc<AtomicBool>,
}

impl Caches {
    /// only get object from memory, not from tmp file
    fn try_get(&self, hash: SHA1) -> Option<Arc<CacheObject>> {
        let mut map = self.lru_cache.lock().unwrap();
        map.get(&hash).map(|x| x.data.clone())
    }

    /// !IMPORTANT: because of the process of pack, the file must be written / be writing before, so it won't be dead lock
    /// fall back to temp to get item. **invoker should ensure the hash is in the cache, or it will block forever**
    fn get_fallback(&self, hash: SHA1) -> io::Result<Arc<CacheObject>> {
        let path = self.generate_temp_path(&self.tmp_path, hash);
        // read from tmp file
        let obj = {
            loop {
                match Self::read_from_temp(&path) {
                    Ok(x) => break x,
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        sleep(std::time::Duration::from_millis(10));
                        continue;
                    }
                    Err(e) => return Err(e), // other error
                }
            }
        };

        let mut map = self.lru_cache.lock().unwrap();
        let obj = Arc::new(obj);
        let mut x = ArcWrapper::new(
            obj.clone(),
            self.complete_signal.clone(),
            Some(self.pool.clone()),
        );
        x.set_store_path(path);
        let _ = map.insert(hash, x); // handle the error
        Ok(obj)
    }

    /// generate the temp file path, hex string of the hash
    fn generate_temp_path(&self, tmp_path: &Path, hash: SHA1) -> PathBuf {
        // This is enough for the original path, 2 chars directory, 40 chars hash, and extra slashes
        let mut path = PathBuf::with_capacity(self.tmp_path.capacity() + SHA1::SIZE * 2 + 5);
        path.push(tmp_path);
        let hash_str = hash._to_string();
        path.push(&hash_str[..2]); // use first 2 chars as the directory
        self.path_prefixes[hash.as_ref()[0] as usize].call_once(|| {
            // Check if the directory exists, if not, create it
            if !path.exists() {
                fs::create_dir_all(&path).unwrap();
            }
        });
        path.push(hash_str);
        path
    }

    fn read_from_temp(path: &Path) -> io::Result<CacheObject> {
        let obj = CacheObject::f_load(path)?;
        // Deserializing will also create an object but without Construction outside and `::new()`
        // So if you want to do sth. while Constructing, impl Deserialize trait yourself
        obj.record_mem_size();
        Ok(obj)
    }

    pub fn queued_tasks(&self) -> usize {
        self.pool.queued_count()
    }

    /// memory used by the index (exclude lru_cache which is contained in CacheObject::get_mem_size())
    pub fn memory_used_index(&self) -> usize {
        self.map_offset.capacity() * (std::mem::size_of::<usize>() + std::mem::size_of::<SHA1>())
            + self.hash_set.capacity() * (std::mem::size_of::<SHA1>())
    }

    /// remove the tmp dir
    pub fn remove_tmp_dir(&self) {
        time_it!("Remove tmp dir", {
            if self.tmp_path.exists() {
                fs::remove_dir_all(&self.tmp_path).unwrap(); //very slow
            }
        });
    }
}

impl _Cache for Caches {
    /// @param size: the size of the memory lru cache. **None means no limit**
    /// @param tmp_path: the path to store the cache object in the tmp file
    fn new(mem_size: Option<usize>, tmp_path: PathBuf, thread_num: usize) -> Self
    where
        Self: Sized,
    {
        // `None` means no limit, so no need to create the tmp dir
        if mem_size.is_some() {
            fs::create_dir_all(&tmp_path).unwrap();
        }

        Caches {
            map_offset: DashMap::new(),
            hash_set: DashSet::new(),
            lru_cache: Mutex::new(LruCache::new(mem_size.unwrap_or(usize::MAX))),
            mem_size,
            tmp_path,
            path_prefixes: [const { Once::new() }; 256],
            pool: Arc::new(ThreadPool::new(thread_num)),
            complete_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    fn get_hash(&self, offset: usize) -> Option<SHA1> {
        self.map_offset.get(&offset).map(|x| *x)
    }

    fn insert(&self, offset: usize, hash: SHA1, obj: CacheObject) -> Arc<CacheObject> {
        let obj_arc = Arc::new(obj);
        {
            // ? whether insert to cache directly or only write to tmp file
            let mut map = self.lru_cache.lock().unwrap();
            let mut a_obj = ArcWrapper::new(
                obj_arc.clone(),
                self.complete_signal.clone(),
                Some(self.pool.clone()),
            );
            if self.mem_size.is_some() {
                a_obj.set_store_path(self.generate_temp_path(&self.tmp_path, hash));
            }
            let _ = map.insert(hash, a_obj);
        }
        //order maters as for reading in 'get_by_offset()'
        self.hash_set.insert(hash);
        self.map_offset.insert(offset, hash);

        obj_arc
    }

    fn get_by_offset(&self, offset: usize) -> Option<Arc<CacheObject>> {
        match self.map_offset.get(&offset) {
            Some(x) => self.get_by_hash(*x),
            None => None,
        }
    }

    fn get_by_hash(&self, hash: SHA1) -> Option<Arc<CacheObject>> {
        // check if the hash is in the cache( lru or tmp file)
        if self.hash_set.contains(&hash) {
            match self.try_get(hash) {
                Some(x) => Some(x),
                None => {
                    if self.mem_size.is_none() {
                        panic!("should not be here when mem_size is not set")
                    }
                    self.get_fallback(hash).ok()
                }
            }
        } else {
            None
        }
    }

    fn total_inserted(&self) -> usize {
        self.hash_set.len()
    }
    fn memory_used(&self) -> usize {
        self.lru_cache.lock().unwrap().current_size() + self.memory_used_index()
    }
    fn clear(&self) {
        time_it!("Caches clear", {
            self.complete_signal.store(true, Ordering::Release);
            self.pool.join();
            self.lru_cache.lock().unwrap().clear();
            self.hash_set.clear();
            self.hash_set.shrink_to_fit();
            self.map_offset.clear();
            self.map_offset.shrink_to_fit();
        });

        assert_eq!(self.pool.queued_count(), 0);
        assert_eq!(self.pool.active_count(), 0);
        assert_eq!(self.lru_cache.lock().unwrap().len(), 0);
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use super::*;
    use crate::{
        hash::SHA1,
        internal::{object::types::ObjectType, pack::cache_object::CacheObjectInfo},
    };

    #[test]
    fn test_cache_single_thread() {
        let source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        let tmp_path = source.clone().join("tests/.cache_tmp");

        if tmp_path.exists() {
            fs::remove_dir_all(&tmp_path).unwrap();
        }

        let cache = Caches::new(Some(2048), tmp_path, 1);
        let a_hash = SHA1::new(String::from("a").as_bytes());
        let b_hash = SHA1::new(String::from("b").as_bytes());
        let a = CacheObject {
            info: CacheObjectInfo::BaseObject(ObjectType::Blob, a_hash),
            data_decompressed: vec![0; 800],
            mem_recorder: None,
            offset: 0,
        };
        let b = CacheObject {
            info: CacheObjectInfo::BaseObject(ObjectType::Blob, b_hash),
            data_decompressed: vec![0; 800],
            mem_recorder: None,
            offset: 0,
        };
        // insert a
        cache.insert(a.offset, a_hash, a.clone());
        assert!(cache.hash_set.contains(&a_hash));
        assert!(cache.try_get(a_hash).is_some());

        // insert b, a should still be in cache
        cache.insert(b.offset, b_hash, b.clone());
        assert!(cache.hash_set.contains(&b_hash));
        assert!(cache.try_get(b_hash).is_some());
        assert!(cache.try_get(a_hash).is_some());

        let c_hash = SHA1::new(String::from("c").as_bytes());
        // insert c which will evict both a and b
        let c = CacheObject {
            info: CacheObjectInfo::BaseObject(ObjectType::Blob, c_hash),
            data_decompressed: vec![0; 1700],
            mem_recorder: None,
            offset: 0,
        };
        cache.insert(c.offset, c_hash, c.clone());
        assert!(cache.try_get(a_hash).is_none());
        assert!(cache.try_get(b_hash).is_none());
        assert!(cache.try_get(c_hash).is_some());
        assert!(cache.get_by_hash(c_hash).is_some());
    }
}
