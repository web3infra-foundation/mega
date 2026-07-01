//! Domain-scoped accessors for monorepo application services.

use std::sync::Arc;

use jupiter::storage::Storage;

use super::{
    context::{
        AdminApplicationService, ClApplicationService, CodeReviewApplicationService,
        ConversationApplicationService, IssueApplicationService, LfsApplicationService,
        NoteApplicationService, ReviewerApplicationService, ServiceContext,
        SidebarApplicationService, UserApplicationService, WebhookApplicationService,
    },
    service::MonoApiService,
    stack::build_mono_stack,
};
use crate::{
    TransportRuntime,
    application::{
        api_service::cache::GitObjectCache,
        artifact::ArtifactApplicationService,
        build_trigger::{BuildTriggerService, ChangesPort, SharedBuildDispatch},
    },
};

/// Bundles monorepo application services for injection into HTTP handlers.
#[derive(Clone)]
pub struct MonoAppServices {
    ctx: ServiceContext,
    cl: ClApplicationService,
    issue: IssueApplicationService,
    conversation: ConversationApplicationService,
    admin: AdminApplicationService,
    user: UserApplicationService,
    lfs: LfsApplicationService,
    artifact: ArtifactApplicationService,
    build_trigger: Option<Arc<BuildTriggerService>>,
    transport: TransportRuntime,
    webhook: WebhookApplicationService,
    sidebar: SidebarApplicationService,
    code_review: CodeReviewApplicationService,
    reviewer: ReviewerApplicationService,
    note: NoteApplicationService,
    git: MonoApiService,
    changes_port: Arc<dyn ChangesPort>,
}

impl MonoAppServices {
    pub fn new(
        storage: Storage,
        git_object_cache: Arc<GitObjectCache>,
        build_dispatch: Option<SharedBuildDispatch>,
    ) -> Self {
        let ctx = ServiceContext::new(storage, git_object_cache, build_dispatch);
        let (git, cl, admin) = build_mono_stack(
            ctx.storage().clone(),
            ctx.git_object_cache().clone(),
            ctx.build_dispatch(),
        );
        let changes_port: Arc<dyn ChangesPort> = Arc::new(cl.clone());
        let transport = TransportRuntime::new(
            ctx.storage().clone(),
            ctx.git_object_cache().clone(),
            git.clone(),
            cl.clone(),
        );
        let build_trigger = ctx.build_dispatch().map(|build_dispatch| {
            Arc::new(BuildTriggerService::new(
                ctx.storage().clone(),
                build_dispatch,
                changes_port.clone(),
            ))
        });
        Self {
            cl: cl.clone(),
            issue: IssueApplicationService::new(ctx.clone()),
            conversation: ConversationApplicationService::new(ctx.clone()),
            admin,
            user: UserApplicationService::new(ctx.clone()),
            lfs: LfsApplicationService::new(ctx.clone()),
            artifact: ArtifactApplicationService::from_storage(ctx.storage()),
            build_trigger,
            transport,
            webhook: WebhookApplicationService::new(ctx.clone()),
            sidebar: SidebarApplicationService::new(ctx.clone()),
            code_review: CodeReviewApplicationService::new(ctx.clone()),
            reviewer: ReviewerApplicationService::new(ctx.clone()),
            note: NoteApplicationService::new(ctx.clone()),
            git,
            changes_port,
            ctx,
        }
    }

    pub fn storage(&self) -> &Storage {
        self.ctx.storage()
    }

    pub fn git_object_cache(&self) -> Arc<GitObjectCache> {
        self.ctx.git_object_cache().clone()
    }

    pub fn build_dispatch(&self) -> Option<SharedBuildDispatch> {
        self.ctx.build_dispatch()
    }

    pub fn transport_runtime(&self) -> &TransportRuntime {
        &self.transport
    }

    pub fn changes_port(&self) -> Arc<dyn ChangesPort> {
        self.changes_port.clone()
    }

    pub fn git(&self) -> &MonoApiService {
        &self.git
    }

    pub fn webhook(&self) -> &WebhookApplicationService {
        &self.webhook
    }

    pub fn sidebar(&self) -> &SidebarApplicationService {
        &self.sidebar
    }

    pub fn code_review(&self) -> &CodeReviewApplicationService {
        &self.code_review
    }

    pub fn reviewer(&self) -> &ReviewerApplicationService {
        &self.reviewer
    }

    pub fn note(&self) -> &NoteApplicationService {
        &self.note
    }

    pub fn cl(&self) -> &ClApplicationService {
        &self.cl
    }

    pub fn issue(&self) -> &IssueApplicationService {
        &self.issue
    }

    pub fn conversation(&self) -> &ConversationApplicationService {
        &self.conversation
    }

    pub fn admin(&self) -> &AdminApplicationService {
        &self.admin
    }

    pub fn user(&self) -> &UserApplicationService {
        &self.user
    }

    pub fn lfs(&self) -> &LfsApplicationService {
        &self.lfs
    }

    pub fn artifact(&self) -> &ArtifactApplicationService {
        &self.artifact
    }

    pub fn build_trigger(&self) -> &BuildTriggerService {
        self.build_trigger
            .as_deref()
            .expect("build trigger requires build_dispatch")
    }
}
