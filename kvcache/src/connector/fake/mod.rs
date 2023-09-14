use super::Connector;
use anyhow::Result;
use std::{cell::RefCell, collections::HashMap, hash::Hash};

pub struct FakeKVstore<K, V> {
    table: RefCell<HashMap<K, V>>,
}
impl<K, V> Connector for FakeKVstore<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    type K = K;

    type V = V;
    fn new() -> Self {
        Self {
            table: RefCell::new(HashMap::new()),
        }
    }
    fn get(&self, key: Self::K) -> Option<Self::V> {
        self.table.borrow().get(&key).cloned()
    }

    fn set(&self, key: Self::K, v: Self::V) -> Result<()> {
        self.table.borrow_mut().insert(key, v);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::FakeKVstore;
    use crate::KVCache;

    #[test]
    fn test_face_connect() {
        let cache = KVCache::<FakeKVstore<_, _>>::new();
        cache.set(3, 65).unwrap();
        cache.set(4, 45).unwrap();
        assert_eq!(cache.get(3), Some(65));
        assert_eq!(cache.get(4), Some(45));
    }
}
