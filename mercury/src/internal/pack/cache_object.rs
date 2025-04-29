use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::{fs, io};
use std::{ops::Deref, sync::Arc};

use lru_mem::{HeapSize, MemSize};
use serde::{Deserialize, Serialize};
use threadpool::ThreadPool;

use crate::internal::pack::entry::Entry;
use crate::internal::pack::utils;
use crate::{hash::SHA1, internal::object::types::ObjectType};

// /// record heap-size of all CacheObjects, used for memory limit.
// static CACHE_OBJS_MEM_SIZE: AtomicUsize = AtomicUsize::new(0);

/// file load&store trait
pub trait FileLoadStore: Serialize + for<'a> Deserialize<'a> {
    fn f_load(path: &Path) -> Result<Self, io::Error>;
    fn f_save(&self, path: &Path) -> Result<(), io::Error>;
}

// trait alias, so that impl FileLoadStore == impl Serialize + Deserialize
impl<T: Serialize + for<'a> Deserialize<'a>> FileLoadStore for T {
    fn f_load(path: &Path) -> Result<T, io::Error> {
        let data = fs::read(path)?;
        let obj: T = bincode::serde::decode_from_slice(&data, bincode::config::standard())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
            .0;
        Ok(obj)
    }
    fn f_save(&self, path: &Path) -> Result<(), io::Error> {
        if path.exists() {
            return Ok(());
        }
        let data = bincode::serde::encode_to_vec(self, bincode::config::standard()).unwrap();
        let path = path.with_extension("temp");
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path.clone())?;
            file.write_all(&data)?;
        }
        let final_path = path.with_extension("");
        fs::rename(&path, final_path.clone())?;
        Ok(())
    }
}

/// Represents the metadata of a cache object, indicating whether it is a delta or not.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub(crate) enum CacheObjectInfo {
    /// The object is one of the four basic types:
    /// [`ObjectType::Blob`], [`ObjectType::Tree`], [`ObjectType::Commit`], or [`ObjectType::Tag`].
    /// The metadata contains the [`ObjectType`] and the [`SHA1`] hash of the object.
    BaseObject(ObjectType, SHA1),
    /// The object is an offset delta with a specified offset delta [`usize`],
    /// and the size of the expanded object (previously `delta_final_size`).
    OffsetDelta(usize, usize),
    /// Similar to [`OffsetDelta`], but delta algorithm is `zstd`.
    OffsetZstdelta(usize, usize),
    /// The object is a hash delta with a specified [`SHA1`] hash,
    /// and the size of the expanded object (previously `delta_final_size`).
    HashDelta(SHA1, usize),
}

impl CacheObjectInfo {
    /// Get the [`ObjectType`] of the object.
    pub(crate) fn object_type(&self) -> ObjectType {
        match self {
            CacheObjectInfo::BaseObject(obj_type, _) => *obj_type,
            CacheObjectInfo::OffsetDelta(_, _) => ObjectType::OffsetDelta,
            CacheObjectInfo::OffsetZstdelta(_, _) => ObjectType::OffsetZstdelta,
            CacheObjectInfo::HashDelta(_, _) => ObjectType::HashDelta,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheObject {
    pub(crate) info: CacheObjectInfo,
    pub offset: usize,
    pub data_decompressed: Vec<u8>,
    pub mem_recorder: Option<Arc<AtomicUsize>>, // record mem-size of all CacheObjects of a Pack
}

impl Clone for CacheObject {
    fn clone(&self) -> Self {
        let obj = CacheObject {
            info: self.info.clone(),
            offset: self.offset,
            data_decompressed: self.data_decompressed.clone(),
            mem_recorder: self.mem_recorder.clone(),
        };
        obj.record_mem_size();
        obj
    }
}

// ! used by lru_mem to calculate the size of the object, limit the memory usage.
// ! the implementation of HeapSize is not accurate, only calculate the size of the data_decompress
// Note that: mem_size == value_size + heap_size, and we only need to impl HeapSize because value_size is known
impl HeapSize for CacheObject {
    /// If a [`CacheObject`] is [`ObjectType::HashDelta`] or [`ObjectType::OffsetDelta`],
    /// it will expand to another [`CacheObject`] of other types. To prevent potential OOM,
    /// we record the size of the expanded object as well as that of the object itself.
    ///
    /// Base objects, *i.e.*, [`ObjectType::Blob`], [`ObjectType::Tree`], [`ObjectType::Commit`],
    /// and [`ObjectType::Tag`], will not be expanded, so the heap-size of the object is the same
    /// as the size of the data.
    ///
    /// See [Comment in PR #755](https://github.com/web3infra-foundation/mega/pull/755#issuecomment-2543100481) for more details.
    fn heap_size(&self) -> usize {
        match &self.info {
            CacheObjectInfo::BaseObject(_, _) => self.data_decompressed.heap_size(),
            CacheObjectInfo::OffsetDelta(_, delta_final_size)
            | CacheObjectInfo::OffsetZstdelta(_, delta_final_size)
            | CacheObjectInfo::HashDelta(_, delta_final_size) => {
                // To those who are concerned about why these two values are added,
                // let's consider the lifetime of two `CacheObject`s, say `delta_obj`
                // and `final_obj` in the function `Pack::rebuild_delta`.
                //
                // `delta_obj` is dropped only after `Pack::rebuild_delta` returns,
                // but the space for `final_obj` is allocated in that function.
                //
                // Therefore, during the execution of `Pack::rebuild_delta`, both `delta_obj`
                // and `final_obj` coexist. The maximum memory usage is the sum of the memory
                // usage of `delta_obj` and `final_obj`.
                self.data_decompressed.heap_size() + delta_final_size
            }
        }
    }
}

impl Drop for CacheObject {
    // Check: the heap-size subtracted when Drop is equal to the heap-size recorded
    // (cannot change the heap-size during life cycle)
    fn drop(&mut self) {
        // (&*self).heap_size() != self.heap_size()
        if let Some(mem_recorder) = &self.mem_recorder {
            mem_recorder.fetch_sub((*self).mem_size(), Ordering::Release);
        }
    }
}

/// Heap-size recorder for a class(struct)
/// <br> You should use a static Var to record mem-size
/// and record mem-size after construction & minus it in `drop()`
/// <br> So, variable-size fields in object should NOT be modified to keep heap-size stable.
/// <br> Or, you can record the initial mem-size in this object
/// <br> Or, update it (not impl)
pub trait MemSizeRecorder: MemSize {
    fn record_mem_size(&self);
    fn set_mem_recorder(&mut self, mem_size: Arc<AtomicUsize>);
    // fn get_mem_size() -> usize;
}

impl MemSizeRecorder for CacheObject {
    /// record the mem-size of this `CacheObj` in a `static` `var`
    /// <br> since that, DO NOT modify `CacheObj` after recording
    fn record_mem_size(&self) {
        if let Some(mem_recorder) = &self.mem_recorder {
            mem_recorder.fetch_add(self.mem_size(), Ordering::Release);
        }
    }

    fn set_mem_recorder(&mut self, mem_recorder: Arc<AtomicUsize>) {
        self.mem_recorder = Some(mem_recorder);
    }

    // fn get_mem_size() -> usize {
    //     CACHE_OBJS_MEM_SIZE.load(Ordering::Acquire)
    // }
}

impl CacheObject {
    /// Create a new CacheObject which is neither [`ObjectType::OffsetDelta`] nor [`ObjectType::HashDelta`].
    pub fn new_for_undeltified(obj_type: ObjectType, data: Vec<u8>, offset: usize) -> Self {
        let hash = utils::calculate_object_hash(obj_type, &data);
        CacheObject {
            info: CacheObjectInfo::BaseObject(obj_type, hash),
            offset,
            data_decompressed: data,
            mem_recorder: None,
        }
    }

    /// Get the [`ObjectType`] of the object.
    pub fn object_type(&self) -> ObjectType {
        self.info.object_type()
    }

    /// Get the [`SHA1`] hash of the object.
    ///
    /// If the object is a delta object, return [`None`].
    pub fn base_object_hash(&self) -> Option<SHA1> {
        match &self.info {
            CacheObjectInfo::BaseObject(_, hash) => Some(*hash),
            _ => None,
        }
    }

    /// Get the offset delta of the object.
    ///
    /// If the object is not an offset delta, return [`None`].
    pub fn offset_delta(&self) -> Option<usize> {
        match &self.info {
            CacheObjectInfo::OffsetDelta(offset, _) => Some(*offset),
            _ => None,
        }
    }

    /// Get the hash delta of the object.
    ///
    /// If the object is not a hash delta, return [`None`].
    pub fn hash_delta(&self) -> Option<SHA1> {
        match &self.info {
            CacheObjectInfo::HashDelta(hash, _) => Some(*hash),
            _ => None,
        }
    }

    /// transform the CacheObject to Entry
    pub fn to_entry(&self) -> Entry {
        match self.info {
            CacheObjectInfo::BaseObject(obj_type, hash) => Entry {
                obj_type,
                data: self.data_decompressed.clone(),
                hash,
            },
            _ => {
                unreachable!("delta object should not persist!")
            }
        }
    }
}

/// trait alias for simple use
pub trait ArcWrapperBounds:
    HeapSize + Serialize + for<'a> Deserialize<'a> + Send + Sync + 'static
{
}
// You must impl `Alias Trait` for all the `T` satisfying Constraints
// Or, `T` will not satisfy `Alias Trait` even if it satisfies the Original traits
impl<T: HeapSize + Serialize + for<'a> Deserialize<'a> + Send + Sync + 'static> ArcWrapperBounds
    for T
{
}

/// Implementing encapsulation of Arc to enable third-party Trait HeapSize implementation for the Arc type
/// Because of use Arc in LruCache, the LruCache is not clear whether a pointer will drop the referenced
/// content when it is ejected from the cache, the actual memory usage is not accurate
pub struct ArcWrapper<T: ArcWrapperBounds> {
    pub data: Arc<T>,
    complete_signal: Arc<AtomicBool>,
    pool: Option<Arc<ThreadPool>>,
    pub store_path: Option<PathBuf>, // path to store when drop
}
impl<T: ArcWrapperBounds> ArcWrapper<T> {
    /// Create a new ArcWrapper
    pub fn new(data: Arc<T>, share_flag: Arc<AtomicBool>, pool: Option<Arc<ThreadPool>>) -> Self {
        ArcWrapper {
            data,
            complete_signal: share_flag,
            pool,
            store_path: None,
        }
    }
    pub fn set_store_path(&mut self, path: PathBuf) {
        self.store_path = Some(path);
    }
}

impl<T: ArcWrapperBounds> HeapSize for ArcWrapper<T> {
    fn heap_size(&self) -> usize {
        self.data.heap_size()
    }
}

impl<T: ArcWrapperBounds> Clone for ArcWrapper<T> {
    /// clone won't clone the store_path
    fn clone(&self) -> Self {
        ArcWrapper {
            data: self.data.clone(),
            complete_signal: self.complete_signal.clone(),
            pool: self.pool.clone(),
            store_path: None,
        }
    }
}

impl<T: ArcWrapperBounds> Deref for ArcWrapper<T> {
    type Target = Arc<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl<T: ArcWrapperBounds> Drop for ArcWrapper<T> {
    // `drop` will be called in `lru_cache.insert()` when cache full & eject the LRU
    // `lru_cache.insert()` is protected by Mutex
    fn drop(&mut self) {
        if !self.complete_signal.load(Ordering::Acquire) {
            if let Some(path) = &self.store_path {
                match &self.pool {
                    Some(pool) => {
                        let data_copy = self.data.clone();
                        let path_copy = path.clone();
                        let complete_signal = self.complete_signal.clone();
                        // block entire process, wait for IO, Control Memory
                        // queue size will influence the Memory usage
                        while pool.queued_count() > 2000 {
                            std::thread::yield_now();
                        }
                        pool.execute(move || {
                            if !complete_signal.load(Ordering::Acquire) {
                                let res = data_copy.f_save(&path_copy);
                                if let Err(e) = res {
                                    println!("[f_save] {:?} error: {:?}", path_copy, e);
                                }
                            }
                        });
                    }
                    None => {
                        let res = self.data.f_save(path);
                        if let Err(e) = res {
                            println!("[f_save] {:?} error: {:?}", path, e);
                        }
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod test {
    use std::{fs, sync::Mutex};

    use lru_mem::LruCache;

    use super::*;
    #[test]
    #[ignore = "only in single thread"]
    // 只在单线程测试
    fn test_heap_size_record() {
        let mut obj = CacheObject {
            info: CacheObjectInfo::BaseObject(ObjectType::Blob, SHA1::default()),
            offset: 0,
            data_decompressed: vec![0; 1024],
            mem_recorder: None,
        };
        let mem = Arc::new(AtomicUsize::default());
        assert_eq!(mem.load(Ordering::Relaxed), 0);
        obj.set_mem_recorder(mem.clone());
        obj.record_mem_size();
        assert_eq!(mem.load(Ordering::Relaxed), 1120);
        drop(obj);
        assert_eq!(mem.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_cache_object_with_same_size() {
        let a = CacheObject {
            info: CacheObjectInfo::BaseObject(ObjectType::Blob, SHA1::default()),
            offset: 0,
            data_decompressed: vec![0; 1024],
            mem_recorder: None,
        };
        assert!(a.heap_size() == 1024);

        // let b = ArcWrapper(Arc::new(a.clone()));
        let b = ArcWrapper::new(Arc::new(a.clone()), Arc::new(AtomicBool::new(false)), None);
        assert!(b.heap_size() == 1024);
    }
    #[test]
    #[ignore]
    fn test_cache_object_with_lru() {
        let mut cache = LruCache::new(2048);

        let hash_a = SHA1::default();
        let hash_b = SHA1::new(b"b"); // whatever different hash
        let a = CacheObject {
            info: CacheObjectInfo::BaseObject(ObjectType::Blob, hash_a),
            offset: 0,
            data_decompressed: vec![0; 1024],
            mem_recorder: None,
        };
        println!("a.heap_size() = {}", a.heap_size());

        let b = CacheObject {
            info: CacheObjectInfo::BaseObject(ObjectType::Blob, hash_b),
            offset: 0,
            data_decompressed: vec![0; (1024.0 * 1.5) as usize],
            mem_recorder: None,
        };
        {
            let r = cache.insert(
                hash_a.to_string(),
                ArcWrapper::new(Arc::new(a.clone()), Arc::new(AtomicBool::new(true)), None),
            );
            assert!(r.is_ok())
        }
        {
            let r = cache.try_insert(
                hash_b.to_string(),
                ArcWrapper::new(Arc::new(b.clone()), Arc::new(AtomicBool::new(true)), None),
            );
            assert!(r.is_err());
            if let Err(lru_mem::TryInsertError::WouldEjectLru { .. }) = r {
                // 匹配到指定错误，不需要额外操作
            } else {
                panic!("Expected WouldEjectLru error");
            }
            // 使用不同的键插入b，这样a会被驱逐
            let r = cache.insert(
                hash_b.to_string(),
                ArcWrapper::new(Arc::new(b.clone()), Arc::new(AtomicBool::new(true)), None),
            );
            assert!(r.is_ok());
        }
        {
            // a should be ejected
            let r = cache.get(&hash_a.to_string());
            assert!(r.is_some());
        }
    }

    #[derive(Serialize, Deserialize)]
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
    #[test]
    fn test_lru_drop() {
        println!("insert a");
        let cache = LruCache::new(2048);
        let cache = Arc::new(Mutex::new(cache));
        {
            let mut c = cache.as_ref().lock().unwrap();
            let _ = c.insert(
                "a",
                ArcWrapper::new(
                    Arc::new(Test { a: 1024 }),
                    Arc::new(AtomicBool::new(true)),
                    None,
                ),
            );
        }
        println!("insert b, a should be ejected");
        {
            let mut c = cache.as_ref().lock().unwrap();
            let _ = c.insert(
                "b",
                ArcWrapper::new(
                    Arc::new(Test { a: 1200 }),
                    Arc::new(AtomicBool::new(true)),
                    None,
                ),
            );
        }
        let b = {
            let mut c = cache.as_ref().lock().unwrap();
            c.get("b").cloned()
        };
        println!("insert c, b should not be ejected");
        {
            let mut c = cache.as_ref().lock().unwrap();
            let _ = c.insert(
                "c",
                ArcWrapper::new(
                    Arc::new(Test { a: 1200 }),
                    Arc::new(AtomicBool::new(true)),
                    None,
                ),
            );
        }
        println!("user b: {}", b.as_ref().unwrap().a);
        println!("test over, enject all");
    }

    #[test]
    fn test_cache_object_serialize() {
        let a = CacheObject {
            info: CacheObjectInfo::BaseObject(ObjectType::Blob, SHA1::default()),
            offset: 0,
            data_decompressed: vec![0; 1024],
            mem_recorder: None,
        };
        let s = bincode::serde::encode_to_vec(&a, bincode::config::standard()).unwrap();
        let b: CacheObject = bincode::serde::decode_from_slice(&s, bincode::config::standard())
            .unwrap()
            .0;
        assert_eq!(a.info, b.info);
        assert_eq!(a.data_decompressed, b.data_decompressed);
        assert_eq!(a.offset, b.offset);
    }

    #[test]
    fn test_arc_wrapper_drop_store() {
        let mut path = PathBuf::from(".cache_temp/test_arc_wrapper_drop_store");
        fs::create_dir_all(&path).unwrap();
        path.push("test_obj");
        let mut a = ArcWrapper::new(Arc::new(1024), Arc::new(AtomicBool::new(false)), None);
        a.set_store_path(path.clone());
        drop(a);

        assert!(path.exists());
        path.pop();
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    /// test warpper can't correctly store the data when lru eject it
    fn test_arc_wrapper_with_lru() {
        let mut cache = LruCache::new(1500);
        let path = PathBuf::from(".cache_temp/test_arc_wrapper_with_lru");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        let shared_flag = Arc::new(AtomicBool::new(false));

        // insert a, a not ejected
        let a_path = path.join("a");
        {
            let mut a = ArcWrapper::new(Arc::new(Test { a: 1024 }), shared_flag.clone(), None);
            a.set_store_path(a_path.clone());
            let b = ArcWrapper::new(Arc::new(1024), shared_flag.clone(), None);
            assert!(b.store_path.is_none());

            println!("insert a with heap size: {:?}", a.heap_size());
            let rt = cache.insert("a", a);
            if let Err(e) = rt {
                panic!("{}", format!("insert a failed: {:?}", e.to_string()));
            }
            println!("after insert a, cache used = {}", cache.current_size());
        }
        assert!(!a_path.exists());

        let b_path = path.join("b");
        // insert b, a should be ejected
        {
            let mut b = ArcWrapper::new(Arc::new(Test { a: 996 }), shared_flag.clone(), None);
            b.set_store_path(b_path.clone());
            let rt = cache.insert("b", b);
            if let Err(e) = rt {
                panic!("{}", format!("insert a failed: {:?}", e.to_string()));
            }
            println!("after insert b, cache used = {}", cache.current_size());
        }
        assert!(a_path.exists());
        assert!(!b_path.exists());
        shared_flag.store(true, Ordering::Release);
        fs::remove_dir_all(path).unwrap();
        // should pass even b's path not exists
    }
}
