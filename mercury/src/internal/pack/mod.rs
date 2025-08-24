//!
//! ## Reference
//! 1. Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
//!
mod cache;
mod cache_object;
mod channel_reader;
mod decode;
mod encode;
pub(crate) mod entry;
mod utils;
mod waitlist;
pub(crate) mod wrapper;

pub use cache_object::CacheObject;
pub use entry::Entry;

use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use threadpool::ThreadPool;

use self::cache::Caches;
use self::waitlist::Waitlist;
use crate::hash::SHA1;
use crate::internal::object::ObjectTrait;

const DEFAULT_TMP_DIR: &str = "./.cache_temp";

/// Collection of objects and supporting data used to build or decode pack files.
pub struct Pack {
    pub number: usize,
    pub signature: SHA1,
    pub objects: Vec<Box<dyn ObjectTrait>>,
    pub pool: Arc<ThreadPool>,
    pub waitlist: Arc<Waitlist>,
    pub caches: Arc<Caches>,
    pub mem_limit: Option<usize>,
    pub cache_objs_mem: Arc<AtomicUsize>, // the memory size of CacheObjects in this Pack
    pub clean_tmp: bool,
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::util::SubscriberInitExt;

    pub(crate) fn init_logger() {
        let _ = tracing_subscriber::fmt::Subscriber::builder()
            .with_target(false)
            .without_time()
            .with_level(true)
            .with_max_level(tracing::Level::DEBUG)
            .finish()
            .try_init(); // avoid multi-init

        // CAUTION: This two is same
        // 1.
        // tracing_subscriber::fmt().init();
        //
        // 2.
        // env::set_var("RUST_LOG", "debug"); // must be set if use `fmt::init()`, or no output
        // tracing_subscriber::fmt::init();
    }
}
