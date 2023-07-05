use super::ObjectT;
use crate::hash::Hash;
use lru::LruCache;
use std::{num::NonZeroUsize, sync::Arc};

#[derive(Hash, Clone, PartialEq, Eq)]
struct OffHash {
    o: usize,
    h: Hash,
}

/// In ObjectCache ,we need the bounds like:
/// - between offset and object data
/// - between hash value and object data
///
/// Cause we use the lru Cache , these two bound should be consistent.
/// So ,build map like this
/// ```text
///     Offset
///            ↘
///             OffHash(usize,hash) → Object
///           ↗
///     Hash
/// ```
pub struct ObjectCache {
    ioffset: LruCache<usize, OffHash>,
    ihash: LruCache<Hash, OffHash>,
    inner: LruCache<OffHash, Arc<dyn ObjectT>>,
}
/// The Size of Object Cache during the decode operation should be talked about.
/// There are --window and --depth options in the process of git pack packaging
///
/// These two options affect how the objects contained in the package are stored
///  using incremental compression. Objects are first internally sorted by type,
/// size, and optional name, and compared to other objects in --window to see if
///  using incremental compression saves space. - Depth limits the maximum depth;
/// making it too deep affects the performance of the unpacking party, as incremental
/// data needs to be applied multiple times to get the necessary objects.
///  --window defaults to 10, --depth is 50.
///
/// So if the options are defaults, the size of cache size should be 10 ~ 50 is ok.
///
/// But After the test, The size "50" also may meet a "cache miss" problem . This Size
/// adjust to 300 more, the decode operation is normal.
/// TODO : deal with "cache miss", get the miss object from DataBase or other sink target.
const CACHE_SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(300) };
impl Default for ObjectCache {
    fn default() -> Self {
        Self {
            ioffset: LruCache::new(CACHE_SIZE),
            ihash: LruCache::new(CACHE_SIZE),
            inner: LruCache::new(CACHE_SIZE),
        }
    }
}
impl ObjectCache {
    pub fn new() -> Self {
        ObjectCache {
            ioffset: LruCache::new(CACHE_SIZE),
            ihash: LruCache::new(CACHE_SIZE),
            inner: LruCache::new(CACHE_SIZE),
        }
    }

    pub fn put(&mut self, offset: usize, hash: Hash, obj: Arc<dyn ObjectT>) {
        let oh: OffHash = OffHash { o: offset, h: hash };
        self.ioffset.put(offset, oh.clone());
        self.ihash.put(hash, oh.clone());
        self.inner.put(oh, Arc::clone(&obj));
    }

    pub fn get(&mut self, offset: usize) -> Option<Arc<dyn ObjectT>> {
        let oh = self.ioffset.get(&offset)?;
        self.ihash.get(&oh.h)?;
        self.inner.get(oh).cloned()
    }

    pub fn get_hash(&mut self, h: Hash) -> Option<Arc<dyn ObjectT>> {
        let oh = self.ihash.get(&h)?;
        self.ioffset.get(&oh.o)?;
        self.inner.get(oh).cloned()
    }
}
#[cfg(test)]
mod test {
    use std::sync::Arc;

    use serde_json::to_vec;

    use super::ObjectCache;
    use crate::{hash::Hash, internal::object::blob};
    #[test] //TODO: to test
    fn test_cache() {
        let mut cache = ObjectCache::new();

        let data = to_vec("sdfsdfsdf").unwrap();
        let h1 = Hash::new(&data);
        cache.put(2, h1, Arc::new(blob::Blob { id: h1, data }));

        let data = to_vec("a222222222222").unwrap();
        let h1 = Hash::new(&data);
        cache.put(3, h1, Arc::new(blob::Blob { id: h1, data }));

        let data = to_vec("33333333").unwrap();
        let h1 = Hash::new(&data);
        cache.put(4, h1, Arc::new(blob::Blob { id: h1, data }));
    }
}
