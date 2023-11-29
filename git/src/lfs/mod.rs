use std::sync::Arc;

use storage::driver::{database::storage::ObjectStorage, file_storage::FileStorage};

pub mod handler;
pub mod lfs_structs;

#[derive(Clone)]
pub struct LfsConfig {
    pub host: String,

    pub port: u16,

    pub storage: Arc<dyn ObjectStorage>,

    pub fs_storage: Arc<dyn FileStorage>,
}
