//!
//!
//!
//!
//!

use std::sync::Arc;

use git::lfs::LfsConfig;
use https_server::AppState;
use storage::driver::file_storage::local_storage::LocalStorage;

mod api_service;
mod git_protocol;
pub mod https_server;
pub mod init;
mod lfs;
mod model;
pub mod ssh_server;

impl From<AppState> for LfsConfig {
    fn from(value: AppState) -> Self {
        Self {
            host: value.options.common.host,
            port: value.options.custom.http_port,
            storage: value.context.storage.clone(),
            fs_storage: Arc::new(LocalStorage::default()),
        }
    }
}

#[cfg(test)]
mod tests {}
