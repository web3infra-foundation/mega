pub mod connector;
pub mod utils;
use std::cell::RefCell;
use connector::Connector;
use anyhow::Result;

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
impl<C> Default for KVCache<C> where C: Connector,{
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
}
