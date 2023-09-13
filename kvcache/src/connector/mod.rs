mod fake;
mod redis;
use anyhow::Result;
use std::cell::RefCell;

pub trait Connector {
    type K;
    type V;
    fn get(&self, key: Self::K) -> Option<Self::V>;
    fn set(&self, key: Self::K, v: Self::V) -> Result<()>;
    fn new() -> Self;
}

#[allow(dead_code)]
pub struct KVCache<C> {
    con: RefCell<C>,
}
#[allow(dead_code)]
impl<C> KVCache<C>
where
    C: Connector,
{
    pub fn new() -> Self {
        KVCache {
            con: RefCell::new(C::new()),
        }
    }

    pub fn get(&self, key: C::K) -> Option<C::V> {
        self.con.borrow().get(key)
    }

    pub fn set(&self, key: C::K, value: C::V) -> Result<()> {
        self.con.borrow_mut().set(key, value)
    }
}
