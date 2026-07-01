//! Shared construction of the Git + CL application service pair.

use std::sync::Arc;

use jupiter::storage::Storage;

use super::{
    context::{AdminApplicationService, ClApplicationService, ServiceContext},
    service::MonoApiService,
};
use crate::{
    application::{api_service::cache::GitObjectCache, build_trigger::SharedBuildDispatch},
    infra::TransportContext,
};

/// Build the canonical `MonoApiService` + `ClApplicationService` pair for a storage context.
pub fn build_mono_stack(
    storage: Storage,
    git_object_cache: Arc<GitObjectCache>,
    build_dispatch: Option<SharedBuildDispatch>,
) -> (
    MonoApiService,
    ClApplicationService,
    AdminApplicationService,
) {
    let ctx = ServiceContext::new(storage, git_object_cache, build_dispatch);
    let git = MonoApiService::new(TransportContext {
        storage: ctx.storage().clone(),
        git_object_cache: ctx.git_object_cache().clone(),
        build_dispatch: ctx.build_dispatch(),
    });
    let admin = AdminApplicationService::new(ctx.clone());
    let cl = ClApplicationService::new(ctx, git.clone(), admin.clone());
    (git, cl, admin)
}
