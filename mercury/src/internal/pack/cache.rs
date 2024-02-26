//!
//!
//!
//!
//!
//!

use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::{fs, io};

use crate::internal::pack::cache_object::{ArcWrapper, CacheObject};
use dashmap::{DashMap, DashSet};
use lru_mem::LruCache;
use threadpool::ThreadPool;
use venus::hash::SHA1;

pub trait _Cache {
    fn new(mem_size: Option<usize>, tmp_path: Option<PathBuf>, thread_num: usize) -> Self
    where
        Self: Sized;
    fn get_hash(&self, offset: usize) -> Option<SHA1>;
    fn insert(&self, offset: usize, hash: SHA1, obj: CacheObject) -> Arc<CacheObject>;
    fn get_by_offset(&self, offset: usize) -> Option<Arc<CacheObject>>;
    fn get_by_hash(&self, h: SHA1) -> Option<Arc<CacheObject>>;
    fn total_inserted(&self) -> usize;
}

#[allow(unused)]
pub struct Caches {
    map_offset: DashMap<usize, SHA1>, // offset to hash
    hash_set: DashSet<SHA1>,          // item in the cache
    lru_cache: Mutex<LruCache<String, ArcWrapper<CacheObject>>>, // *lru_cache reqiure the key to implement lru::MemSize trait, so didn't use SHA1 as the key*
    mem_size: usize,
    tmp_path: PathBuf,
    pool: ThreadPool,
}

impl Caches {
    /// only get object from memory, not from tmp file
    fn try_get(&self, hash: SHA1) -> Option<Arc<CacheObject>> {
        let mut map = self.lru_cache.lock().unwrap();
        map.get(&hash.to_plain_str()).map(|x| x.clone().0)
    }

    /// !IMPORTANT: because of the process of pack, the file must be written / be writing before, so it won't be dead lock
    /// block to get cache item. **invoker should ensure the hash is in the cache, or it will block forever**
    fn get_without_check(&self, hash: SHA1) -> io::Result<Arc<CacheObject>> {
        if let Some(obj) = self.try_get(hash) {
            return Ok(obj);
        }

        // read from tmp file
        let obj = {
            loop {
                match self.read_from_tmp(hash) {
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
        let x = ArcWrapper(Arc::new(obj)); //TODO 挪出锁外
        let _ = map.insert(hash.to_plain_str(), x.clone()); // handle the error
        Ok(x.0)
    }

    /// generate the tmp file path, hex string of the hash
    fn generate_tmp_path(tmp_path: &Path, hash: SHA1) -> PathBuf {
        let mut path = tmp_path.to_path_buf();
        path.push(hash.to_plain_str());
        path
    }

    fn read_from_tmp(&self, hash: SHA1) -> io::Result<CacheObject> {
        let path = Self::generate_tmp_path(&self.tmp_path, hash);
        let b = fs::read(path)?;
        let obj: CacheObject =
            bincode::deserialize(&b).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(obj)
    }

    /// write the object to tmp file,
    /// ! because the file won't be changed after the object is written, use atomic write will ensure thread safety
    // todo use another thread to do this latter
    fn write_to_tmp(tmp_path: &Path, hash: SHA1, obj: &CacheObject) -> io::Result<()> {
        let path = Self::generate_tmp_path(tmp_path, hash);
        let b = bincode::serialize(&obj).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let path = path.with_extension("temp");
        fs::write(path.clone(), b)?;
        let final_path = path.with_extension("");
        fs::rename(path, final_path)?;
        Ok(())
    }
}

impl _Cache for Caches {
    /// @param size: the size of the memory lru cache,
    /// @param tmp_path: the path to store the cache object in the tmp file
    fn new(mem_size: Option<usize>, tmp_path: Option<PathBuf>, thread_num: usize) -> Self
    where
        Self: Sized,
    {
        let tmp_path = tmp_path.unwrap_or(PathBuf::from(".cache_tmp/"));
        fs::create_dir_all(&tmp_path).unwrap();
        println!("tmp_path = {:?}", tmp_path.canonicalize().unwrap());
        Caches {
            map_offset: DashMap::new(),
            hash_set: DashSet::new(),
            lru_cache: Mutex::new(LruCache::new(mem_size.unwrap_or(0))),
            mem_size: mem_size.unwrap_or(0),
            tmp_path,
            pool: ThreadPool::new(thread_num),
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
            let _ = map.insert(hash.to_plain_str(), ArcWrapper(obj_arc.clone()));
        }
        //order maters as for reading in 'get_by_offset()'
        self.hash_set.insert(hash);
        self.map_offset.insert(offset, hash);

        let tmp_path = self.tmp_path.clone();
        let obj_clone = obj_arc.clone();
        self.pool.execute(move || {
            Self::write_to_tmp(&tmp_path, hash, &obj_clone).unwrap();
        });

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
            match self.get_without_check(hash) {
                Ok(obj) => Some(obj),
                Err(_) => {
                    panic!("cache error!");
                }
            }
        } else {
            None
        }
    }

    fn total_inserted(&self) -> usize {
        self.hash_set.len()
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
        let cache = Caches::new(Some(2048), Some(source.clone().join("tests/.cache_tmp")), 1);
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
