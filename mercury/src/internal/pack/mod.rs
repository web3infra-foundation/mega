//! 
//! ## Reference
//! 1. Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
//!
pub mod decode;
pub mod encode;
pub mod wrapper;
pub mod utils;
pub mod cache;


use venus::hash::SHA1;
use threadpool::ThreadPool;
use std::sync::Arc;
use dashmap::DashMap;
use venus::internal::object::ObjectTrait;
use crate::internal::pack::cache::CacheObject;

///
/// 
/// 
#[allow(unused)]
pub struct Pack {
    pub number: usize,
    pub signature: SHA1,
    pub objects: Vec<Box<dyn ObjectTrait>>,
    pub pool: ThreadPool,
    pub waitlist_offset: Arc<DashMap<usize, Vec<CacheObject>>>,
    pub waitlist_ref: Arc<DashMap<SHA1, Vec<CacheObject>>>,
}

#[cfg(test)]
mod tests {
}