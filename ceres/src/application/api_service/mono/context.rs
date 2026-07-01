//! Shared context and domain-scoped application services.

use std::sync::Arc;

use jupiter::storage::Storage;

use crate::{
    application::{api_service::cache::GitObjectCache, build_trigger::SharedBuildDispatch},
    infra::TransportContext,
};
#[derive(Clone)]
pub(crate) struct ServiceContext {
    storage: Storage,
    git_object_cache: Arc<GitObjectCache>,
    build_dispatch: Option<SharedBuildDispatch>,
}

impl ServiceContext {
    pub(crate) fn from_transport(ctx: TransportContext) -> Self {
        Self {
            storage: ctx.storage,
            git_object_cache: ctx.git_object_cache,
            build_dispatch: ctx.build_dispatch,
        }
    }

    pub(crate) fn new(
        storage: Storage,
        git_object_cache: Arc<GitObjectCache>,
        build_dispatch: Option<SharedBuildDispatch>,
    ) -> Self {
        Self {
            storage,
            git_object_cache,
            build_dispatch,
        }
    }

    pub(crate) fn storage(&self) -> &Storage {
        &self.storage
    }

    pub(crate) fn git_object_cache(&self) -> &Arc<GitObjectCache> {
        &self.git_object_cache
    }

    pub(crate) fn build_dispatch(&self) -> Option<SharedBuildDispatch> {
        self.build_dispatch.clone()
    }
}

macro_rules! app_service {
    ($(#[$meta:meta])* $vis:vis struct $name:ident) => {
        $(#[$meta])*
        #[derive(Clone)]
        $vis struct $name {
            pub(super) ctx: ServiceContext,
        }

        impl $name {
            pub(crate) fn new(ctx: ServiceContext) -> Self {
                Self { ctx }
            }
        }
    };
}

app_service! {
    /// Issue and label operations.
    pub struct IssueApplicationService
}
app_service! {
    /// Conversation and comment operations.
    pub struct ConversationApplicationService
}
app_service! {
    /// Admin operations (groups, bots, permissions).
    pub struct AdminApplicationService
}
app_service! {
    /// User profile, tokens, and notification preferences.
    pub struct UserApplicationService
}
app_service! {
    /// Git LFS operations.
    pub struct LfsApplicationService
}

/// Change-list (CL) operations.
#[derive(Clone)]
pub struct ClApplicationService {
    pub(super) ctx: ServiceContext,
    git: super::service::MonoApiService,
    admin: AdminApplicationService,
}

app_service! {
    /// Webhook CRUD operations.
    pub struct WebhookApplicationService
}

impl WebhookApplicationService {
    pub(crate) fn storage(&self) -> &Storage {
        self.ctx.storage()
    }
}
app_service! {
    /// Dynamic sidebar menu operations.
    pub struct SidebarApplicationService
}
app_service! {
    /// Code review thread/comment operations.
    pub struct CodeReviewApplicationService
}
app_service! {
    /// CL reviewer operations.
    pub struct ReviewerApplicationService
}
app_service! {
    /// Note sync operations.
    pub struct NoteApplicationService
}

impl ClApplicationService {
    pub(crate) fn new(
        ctx: ServiceContext,
        git: super::service::MonoApiService,
        admin: AdminApplicationService,
    ) -> Self {
        Self { ctx, git, admin }
    }

    pub(crate) fn git(&self) -> &super::service::MonoApiService {
        &self.git
    }

    pub(crate) fn git_ops(&self) -> &dyn super::git_ops::GitOpsPort {
        &self.git
    }

    pub(crate) fn storage(&self) -> &Storage {
        self.ctx.storage()
    }

    pub(crate) fn admin(&self) -> &AdminApplicationService {
        &self.admin
    }
}
