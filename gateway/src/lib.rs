//!
//!
//!
//!
//!

use std::sync::Arc;

use git::lfs::LfsConfig;
use https::AppState;
use storage::driver::file_storage::local_storage::LocalStorage;

mod api_service;
mod git_protocol;
pub mod https;
pub mod init;
mod lfs;
mod model;
pub mod ssh;

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
