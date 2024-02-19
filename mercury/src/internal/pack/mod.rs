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

///
/// 
/// 
#[allow(unused)]
pub struct Pack {
    pub number: usize,
    pub signature: SHA1,
    pub pool: ThreadPool
}

#[cfg(test)]
mod tests {
}