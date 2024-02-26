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
use venus::internal::object::ObjectTrait;
use crate::internal::pack::waitlist::Waitlist;

///
/// 
/// 
#[allow(unused)]
pub struct Pack {
    pub number: usize,
    pub signature: SHA1,
    pub objects: Vec<Box<dyn ObjectTrait>>,
    pub pool: Arc<ThreadPool>,
    pub waitlist: Arc<Waitlist>
}

#[cfg(test)]
mod tests {
}