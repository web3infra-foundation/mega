use std::sync::Arc;

use jupiter::storage::Storage;

use super::cache::GitObjectCache;

/// Transport-agnostic handles shared by Git transport handlers and application services.
#[derive(Clone)]
pub struct TransportContext {
    pub storage: Storage,
    pub git_object_cache: Arc<GitObjectCache>,
}

impl TransportContext {
    pub fn new(storage: Storage, git_object_cache: Arc<GitObjectCache>) -> Self {
        Self {
            storage,
            git_object_cache,
        }
    }
}
