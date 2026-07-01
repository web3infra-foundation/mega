use std::sync::Arc;

use jupiter::storage::Storage;

use super::cache::GitObjectCache;
use crate::application::build_trigger::SharedBuildDispatch;

/// Transport-agnostic handles shared by Git transport handlers and application services.
#[derive(Clone)]
pub struct TransportContext {
    pub storage: Storage,
    pub git_object_cache: Arc<GitObjectCache>,
    pub build_dispatch: Option<SharedBuildDispatch>,
}

impl TransportContext {
    pub fn new(storage: Storage, git_object_cache: Arc<GitObjectCache>) -> Self {
        Self {
            storage,
            git_object_cache,
            build_dispatch: None,
        }
    }

    pub fn with_build_dispatch(
        storage: Storage,
        git_object_cache: Arc<GitObjectCache>,
        build_dispatch: SharedBuildDispatch,
    ) -> Self {
        Self {
            storage,
            git_object_cache,
            build_dispatch: Some(build_dispatch),
        }
    }
}
