use std::{path::PathBuf, sync::Arc};

use storage::driver::database::storage::ObjectStorage;

pub mod http;

#[derive(Clone)]
pub struct LfsConfig {
    pub host: String,

    pub port: u16,

    pub lfs_content_path: PathBuf,

    pub storage: Arc<dyn ObjectStorage>,
}
