//!
//!
//!
//!
//!

use std::sync::Arc;

use git::lfs::LfsConfig;
use https::AppState;
use storage::driver::file_storage::local_storage::LocalStorage;

pub mod https;
pub mod init;
pub mod ssh;

mod api_service;
mod model;

impl From<AppState> for LfsConfig {
    fn from(value: AppState) -> Self {
        Self {
            host: value.options.host,
            port: value.options.port,
            storage: value.storage,
            fs_storage: Arc::new(LocalStorage::default()),
        }
    }
}

#[cfg(test)]
mod tests {}
