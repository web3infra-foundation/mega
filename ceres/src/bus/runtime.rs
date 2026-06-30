use std::sync::Arc;

use jupiter::storage::Storage;

use super::handler::ApplicationEventHandler;
use crate::{code_edit::post_receive::RuntimeApplicationHandler, infra::cache::GitObjectCache};

#[derive(Clone)]
pub struct TransportRuntime {
    pub storage: Storage,
    pub git_object_cache: Arc<GitObjectCache>,
    pub application: Arc<dyn ApplicationEventHandler>,
}

impl TransportRuntime {
    pub fn new(storage: Storage, git_object_cache: Arc<GitObjectCache>) -> Self {
        let application: Arc<dyn ApplicationEventHandler> = Arc::new(
            RuntimeApplicationHandler::new(storage.clone(), git_object_cache.clone()),
        );
        Self {
            storage,
            git_object_cache,
            application,
        }
    }
}
