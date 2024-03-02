//!
//!
//!
//!
//!
//!

use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::{fs, io};

use crate::internal::pack::cache_object::{ArcWrapper, CacheObject, HeapSizeRecorder};
use crate::time_it;
use dashmap::{DashMap, DashSet};
use lru_mem::LruCache;
use threadpool::ThreadPool;
use venus::hash::SHA1;

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

#[allow(unused)]
pub struct Caches {
    map_offset: DashMap<usize, SHA1>, // offset to hash
    hash_set: DashSet<SHA1>,          // item in the cache
    // dropping large lru cache will take a long time on Windows without multi-thread IO
    // because "multi-thread IO" clone Arc<CacheObject>, so it won't be dropped in the main thread,
    // and `CacheObjects` will be killed by OS after Process ends abnormally
    // Solution: use `mimalloc`
    lru_cache: Mutex<LruCache<String, ArcWrapper<CacheObject>>>, // *lru_cache require the key to implement lru::MemSize trait, so didn't use SHA1 as the key*
    mem_size: Option<usize>,
    tmp_path: PathBuf,
    pool: ThreadPool,
    complete_signal: Arc<AtomicBool>,
}

impl Caches {
    /// only get object from memory, not from tmp file
    fn try_get(&self, hash: SHA1) -> Option<Arc<CacheObject>> {
        let mut map = self.lru_cache.lock().unwrap();
        map.get(&hash.to_plain_str()).map(|x| x.data.clone())
    }

    /// !IMPORTANT: because of the process of pack, the file must be written / be writing before, so it won't be dead lock
    /// fall back to temp to get item. **invoker should ensure the hash is in the cache, or it will block forever**
    fn get_fallback(&self, hash: SHA1) -> io::Result<Arc<CacheObject>> {
        // read from tmp file
        let obj = {
            loop {
                match self.read_from_temp(hash) {
                    Ok(x) => break x,
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        sleep(std::time::Duration::from_millis(10)); //TODO 有没有更好办法
                        continue;
                    }
                    Err(e) => return Err(e), // other error
                }
            }
        };

        let mut map = self.lru_cache.lock().unwrap();
        let obj = Arc::new(obj);
        let mut x = ArcWrapper::new(obj.clone(), self.complete_signal.clone());
        x.set_store_path(Caches::generate_temp_path(&self.tmp_path, hash));
        let _ = map.insert(hash.to_plain_str(), x); // handle the error
        Ok(obj)
    }

    /// generate the temp file path, hex string of the hash
    fn generate_temp_path(tmp_path: &Path, hash: SHA1) -> PathBuf {
        let mut path = tmp_path.to_path_buf();
        path.push(hash.to_plain_str());
        path
    }

    fn read_from_temp(&self, hash: SHA1) -> io::Result<CacheObject> {
        let path = Self::generate_temp_path(&self.tmp_path, hash);
        let b = fs::read(path)?;
        let obj: CacheObject =
            bincode::deserialize(&b).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        // Deserializing will also create an object but without Construction outside and `::new()`
        // So if you want to do sth. while Constructing, impl Deserialize trait yourself
        obj.record_heap_size();
        Ok(obj)
    }

    /// write the object to tmp file,
    /// ! because the file won't be changed after the object is written, use atomic write will ensure thread safety
    // todo use another thread to do this latter
    fn write_to_temp(tmp_path: &Path, hash: SHA1, obj: &CacheObject) -> io::Result<()> {
        let path = Self::generate_temp_path(tmp_path, hash);
        let b = bincode::serialize(&obj).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let path = path.with_extension("temp");
        fs::write(path.clone(), b)?;
        let final_path = path.with_extension("");
        fs::rename(path, final_path)?;
        Ok(())
    }

    pub fn queued_tasks(&self) -> usize {
        self.pool.queued_count()
    }
}

impl _Cache for Caches {
    /// @param size: the size of the memory lru cache. **None means no limit**
    /// @param tmp_path: the path to store the cache object in the tmp file
    fn new(mem_size: Option<usize>, tmp_path: PathBuf, thread_num: usize) -> Self
    where
        Self: Sized,
    {
        fs::create_dir_all(&tmp_path).unwrap();

        Caches {
            map_offset: DashMap::new(),
            hash_set: DashSet::new(),
            lru_cache: Mutex::new(LruCache::new(mem_size.unwrap_or(usize::MAX))),
            mem_size,
            tmp_path,
            pool: ThreadPool::new(thread_num),
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
            let mut a_obj = ArcWrapper::new(obj_arc.clone(), self.complete_signal.clone());
            a_obj.set_store_path(Caches::generate_temp_path(&self.tmp_path, hash));
            let _ = map.insert(hash.to_plain_str(), a_obj);
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
                    match self.get_fallback(hash) {
                        Ok(x) => Some(x),
                        Err(_) => None,
                    }
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
        self.lru_cache.lock().unwrap().current_size()
    }
    fn clear(&self) {
        time_it!("Caches clear", {
            self.complete_signal
                .store(true, std::sync::atomic::Ordering::Relaxed);
            self.pool.join();
            self.lru_cache.lock().unwrap().clear();
            self.hash_set.clear();
            self.map_offset.clear();
        });

        time_it!("Remove tmp dir", {
            fs::remove_dir_all(&self.tmp_path).unwrap(); //very slow
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
    use venus::hash::SHA1;

    #[test]
    fn test_cach_single_thread() {
        let source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        let cache = Caches::new(Some(2048), source.clone().join("tests/.cache_tmp"), 1);
        let a = CacheObject {
            data_decompress: vec![0; 1024],
            hash: SHA1::new(&String::from("a").into_bytes()),
            ..Default::default()
        };
        let b = CacheObject {
            data_decompress: vec![0; 1636],
            hash: SHA1::new(&String::from("b").into_bytes()),
            ..Default::default()
        };
        // insert a
        cache.insert(a.offset, a.hash, a.clone());
        assert!(cache.hash_set.contains(&a.hash));
        assert!(cache.try_get(a.hash).is_some());

        // insert b and make a invalidate
        cache.insert(b.offset, b.hash, b.clone());
        assert!(cache.hash_set.contains(&b.hash));
        assert!(cache.try_get(b.hash).is_some());
        assert!(cache.try_get(a.hash).is_none());

        // get a and make b invalidate
        let _ = cache.get_by_hash(a.hash);
        assert!(cache.try_get(a.hash).is_some());
        assert!(cache.try_get(b.hash).is_none());

        // insert too large c, a will still be in the cache
        let c = CacheObject {
            data_decompress: vec![0; 2049],
            hash: SHA1::new(&String::from("c").into_bytes()),
            ..Default::default()
        };
        cache.insert(c.offset, c.hash, c.clone());
        assert!(cache.try_get(a.hash).is_some());
        assert!(cache.try_get(b.hash).is_none());
        assert!(cache.try_get(c.hash).is_none());
        assert!(cache.get_by_hash(c.hash).is_some());
    }
}
