pub mod fake;
pub mod redis;
use anyhow::Result;


pub trait Connector {
    type K;
    type V;
    fn get(&self, key: Self::K) -> Option<Self::V>;
    fn set(&self, key: Self::K, v: Self::V) -> Result<()>;
    fn del(&self, key: Self::K) -> Result<()>;
    fn new() -> Self;
}

