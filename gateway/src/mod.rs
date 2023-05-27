//!
//!
//!
//!
//!

use std::sync::Arc;

use git::lfs::LfsConfig;
use https::HttpOptions;
use storage::driver::mysql::storage::MysqlStorage;
pub mod https;
pub mod lfs;
pub mod ssh;

impl From<HttpOptions> for LfsConfig {
    fn from(value: HttpOptions) -> Self {
        Self {
            host: value.host,
            port: value.port,
            lfs_content_path: value.lfs_content_path,
            storage: Arc::new(MysqlStorage::default()),
        }
    }
}

#[cfg(test)]
mod tests {}
