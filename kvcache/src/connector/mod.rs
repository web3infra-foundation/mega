pub mod fake;
pub mod redis;
use anyhow::Result;


pub trait Connector {
    type K;
    type V;
    fn get(&self, key: Self::K) -> Option<Self::V>;
    fn set(&self, key: Self::K, v: Self::V) -> Result<()>;
    fn new() -> Self;
}

