//!
//!
//!
//!
//!

use std::sync::Arc;

use git::lfs::LfsConfig;
use https::HttpOptions;
use storage::driver::database::mysql_storage::MysqlStorage;
use webhook::WebhookOptions;
mod api_service;
pub mod https;
pub mod init;
mod model;
pub mod ssh;
pub mod webhook;

impl From<HttpOptions> for LfsConfig {
    fn from(value: HttpOptions) -> Self {
        Self {
            host: value.host,
            port: value.port,
            lfs_content_path: value.lfs_content_path.unwrap(),
            storage: Arc::new(MysqlStorage::default()),
        }
    }
}

impl From<WebhookOptions> for LfsConfig {
    fn from(value: WebhookOptions) -> Self {
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
