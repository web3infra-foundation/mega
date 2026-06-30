//! Domain-scoped accessors for [`MonoApiService`] (gradual split entry point).

use std::sync::Arc;

use jupiter::storage::Storage;

use super::service::MonoApiService;
use crate::infra::TransportContext;

/// Bundles monorepo application services for injection into HTTP handlers.
#[derive(Clone)]
pub struct MonoAppServices {
    inner: MonoApiService,
}

impl MonoAppServices {
    pub fn new(
        storage: Storage,
        git_object_cache: Arc<crate::application::api_service::cache::GitObjectCache>,
    ) -> Self {
        Self {
            inner: MonoApiService::new(TransportContext::new(storage, git_object_cache)),
        }
    }

    pub fn monorepo(&self) -> &MonoApiService {
        &self.inner
    }

    pub fn cl(&self) -> &MonoApiService {
        &self.inner
    }

    pub fn issue(&self) -> &MonoApiService {
        &self.inner
    }

    pub fn conversation(&self) -> &MonoApiService {
        &self.inner
    }

    pub fn admin(&self) -> &MonoApiService {
        &self.inner
    }

    pub fn user(&self) -> &MonoApiService {
        &self.inner
    }
}

impl From<MonoAppServices> for MonoApiService {
    fn from(services: MonoAppServices) -> Self {
        services.inner
    }
}

impl From<&MonoAppServices> for MonoApiService {
    fn from(services: &MonoAppServices) -> Self {
        services.inner.clone()
    }
}
