use dashmap::DashMap;
use venus::hash::SHA1;
use crate::internal::pack::cache::CacheObject;

/// Waitlist for Delta objects while the Base object is not ready.
/// Easier and faster than Channels.
#[derive(Default)]
pub struct Waitlist {
    map_offset: DashMap<usize, Vec<CacheObject>>,
    map_ref: DashMap<SHA1, Vec<CacheObject>>,
}

impl Waitlist {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_offset(&self, offset: usize, obj: CacheObject) {
        self.map_offset.entry(offset).or_insert(Vec::new()).push(obj);
    }

    pub fn insert_ref(&self, hash: SHA1, obj: CacheObject) {
        self.map_ref.entry(hash).or_insert(Vec::new()).push(obj);
    }

    /// Take objects out (get & remove)
    /// <br> Return Vec::new() if None
    pub fn take(&self, offset: usize, hash: SHA1) -> Vec<CacheObject> {
        let mut res = Vec::new();
        if let Some((_, vec)) = self.map_offset.remove(&offset) {
            res.extend(vec);
        }
        if let Some((_, vec)) = self.map_ref.remove(&hash) {
            res.extend(vec);
        }
        res
    }
}