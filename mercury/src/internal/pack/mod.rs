//! 
//! ## Reference
//! 1. Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
//!
pub mod decode;
pub mod encode;
pub mod wrapper;
pub mod utils;
pub mod cache;
pub mod waitlist;
pub mod cache_object;

use venus::hash::SHA1;
use threadpool::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use venus::internal::object::ObjectTrait;
use crate::internal::pack::waitlist::Waitlist;

use self::cache::Caches;

#[allow(unused)]
pub struct Pack {
    pub number: usize,
    pub signature: SHA1,
    pub objects: Vec<Box<dyn ObjectTrait>>,
    pub pool: Arc<ThreadPool>,
    pub waitlist: Arc<Waitlist>,
    pub caches: Arc<Caches>,
    pub mem_limit: usize,
    pub cache_objs_mem: Arc<AtomicUsize>, // the memory size of CacheObjects in this Pack
    pub clean_tmp: bool,
}

#[cfg(test)]
mod tests {
}