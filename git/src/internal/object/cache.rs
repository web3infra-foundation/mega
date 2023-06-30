use super::ObjectT;
use crate::hash::Hash;
use lru::LruCache;
use std::{num::NonZeroUsize, sync::Arc};

#[derive(Hash, Clone, PartialEq, Eq)]
struct OffHash {
    o: usize,
    h: Hash,
}

pub struct ObjectCache {
    ioffset: LruCache<usize, OffHash>,
    ihash: LruCache<Hash, OffHash>,
    inner: LruCache<OffHash, Arc<dyn ObjectT>>,
}

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
