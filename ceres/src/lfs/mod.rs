use std::sync::Arc;

use jupiter::{context::Context, raw_storage::RawStorage};

pub mod handler;
pub mod lfs_structs;

#[derive(Clone)]
pub struct LfsConfig {
    pub host: String,

    pub port: u16,

    pub context: Context,

    pub lfs_storage: Arc<dyn RawStorage>,

    pub repo_name: String,
}
