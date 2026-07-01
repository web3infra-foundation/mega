use std::sync::Arc;

use jupiter::storage::Storage;

use super::handler::ApplicationEventHandler;
use crate::application::{
    api_service::{
        cache::GitObjectCache,
        mono::{ClApplicationService, MonoApiService},
    },
    code_edit::post_receive::RuntimeApplicationHandler,
};

#[derive(Clone)]
pub struct TransportRuntime {
    pub storage: Storage,
    pub git_object_cache: Arc<GitObjectCache>,
    pub application: Arc<dyn ApplicationEventHandler>,
}

impl TransportRuntime {
    pub fn new(
        storage: Storage,
        git_object_cache: Arc<GitObjectCache>,
        git: MonoApiService,
        cl: ClApplicationService,
    ) -> Self {
        let application: Arc<dyn ApplicationEventHandler> =
            Arc::new(RuntimeApplicationHandler::new(git, cl));
        Self {
            storage,
            git_object_cache,
            application,
        }
    }
}
