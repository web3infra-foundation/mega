use crate::hash::Hash;
use lru::LruCache;
use std::{collections::HashMap, num::NonZeroUsize, cell::RefCell};

#[derive(Hash, Clone, PartialEq, Eq)]
struct OffHash {
    o: usize,
    h: Hash,
}

pub trait _Cache {
    type T ;
    fn new(size: Option<usize>) -> Self where Self: Sized;
    fn get_hash(&self, offset: usize) -> Option<Hash>;
    fn get(&self, offset: usize) -> Option<Self::T>;
    fn put(&self, offset: usize, hash: Hash, obj: Self::T);
    fn get_by_hash(&self, h: Hash) -> Option<Self::T>;
}

//pub type  OBCacje<T>  =  ObjectCache<T> ;

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
pub struct ObjectCache<T> {
    ioffset: RefCell<HashMap<usize, OffHash>> ,
    ihash: RefCell<LruCache<Hash, OffHash>>,
    inner: RefCell<LruCache<OffHash, T>>,
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
const CACHE_SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(1000) };
impl<T> Default for ObjectCache<T> {
    fn default() -> Self {
        Self {
            ioffset: RefCell::new(HashMap::new()),
            ihash: RefCell::new(LruCache::new(CACHE_SIZE)),
            inner: RefCell::new(LruCache::new(CACHE_SIZE)),
        }
    }
}
impl<T> _Cache for  ObjectCache<T>
where
    T: Clone,
{
    type T = T; 
    fn new(size: Option<usize>) -> Self {
        let lru_size = if let Some(size) = size {
            NonZeroUsize::new(size).unwrap()
        } else {
            CACHE_SIZE
        };
        ObjectCache {
            ioffset: RefCell::new(HashMap::new()),
            ihash: RefCell::new(LruCache::new(lru_size)),
            inner: RefCell::new(LruCache::new(lru_size)),
        }
    }
    fn get_hash(&self, offset: usize) -> Option<Hash> {
        self.ioffset.borrow().get(&offset).map(|oh| oh.h)
    }
    fn put(&self, offset: usize, hash: Hash, obj: T) {
        let oh: OffHash = OffHash { o: offset, h: hash };
        self.ioffset.borrow_mut().insert(offset, oh.clone());
        self.ihash.borrow_mut().put(hash, oh.clone());
        self.inner.borrow_mut().put(oh, obj);
    }

    fn get(&self, offset: usize) -> Option<T> {
        let binding = self.ioffset.borrow();
        let oh = binding.get(&offset)?;
        self.ihash.borrow_mut().get(&oh.h)?;
        self.inner.borrow_mut().get(oh).cloned()
    }

    fn get_by_hash(&self, h: Hash) -> Option<T> {
        let mut binding = self.ihash.borrow_mut();
        let oh = binding.get(&h)?;
        self.inner.borrow_mut().get(oh).cloned()
    }

    
}

pub mod kvstore{
    use std::cell::RefCell;
    use std::collections::HashMap;
    use crate::internal::pack::Hash;
    //use kvcache::connector::fake::FakeKVstore;
    use kvcache::connector::redis::RedisClient;
    use kvcache::KVCache;
    use super:: _Cache;

    pub struct ObjectCache<T> {
        ioffset: RefCell<HashMap<usize, Hash>>,
        inner : KVCache<RedisClient<Hash,T>>
    }
    impl<T> Default for ObjectCache<T> where T : redis::ToRedisArgs + redis::FromRedisValue + Clone {
        fn default() -> Self {
            Self {
                ioffset: RefCell::new(HashMap::new()),
                inner: KVCache::new(),
            }
        }
    }
    impl<T> _Cache for  ObjectCache<T>
    where
        T: Clone + redis::ToRedisArgs + redis::FromRedisValue ,
    {
        type T = T; 
        fn new(_size: Option<usize>) -> Self {
           Self::default()
        }
        fn get_hash(&self, offset: usize) -> Option<Hash> {
            self.ioffset.borrow().get(&offset).copied()
        }
        fn put(& self, offset: usize, hash: Hash, obj: T) {
            self.ioffset.borrow_mut().insert(offset, hash);
            self.inner.set(hash, obj).unwrap();
        }
    
        fn get(& self, offset: usize) -> Option<T> {
            let binding = self.ioffset.borrow();
            let h = binding.get(&offset)?;
            self.inner.get(*h)    
        }
    
        fn get_by_hash(& self, h: Hash) -> Option<T> {
            self.inner.get(h)  
        }
    
        
    }
    
}


#[cfg(test)]
mod test {
    use std::sync::Arc;

    use serde_json::to_vec;

    use super::{ObjectCache, _Cache};
    use crate::{hash::Hash, internal::object::blob};
    #[test] //TODO: to test
    fn test_cache() {
        let cache = ObjectCache::new(None);

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
