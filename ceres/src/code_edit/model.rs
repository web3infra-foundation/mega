use std::sync::Arc;

use bellatrix::Bellatrix;
use callisto::{entity_ext::generate_link, mega_cl, mega_refs, sea_orm_active_enums::ConvTypeEnum};
use common::errors::MegaError;
use git_internal::internal::object::commit::Commit;
use jupiter::{
    service::reviewer_service::ReviewerService,
    storage::{Storage, mono_storage::MonoStorage},
    utils::converter::FromMegaModel,
};

use crate::{
    api_service::{ApiHandler, cache::GitObjectCache},
    build_trigger::{BuildTriggerService, TriggerContext},
    code_edit::{model, utils as edit_utils},
    merge_checker::CheckerRegistry,
};

pub(crate) trait ConversationMessageFormater {
    fn format(
        &self,
        cl: &mega_cl::Model,
        from_hash: &str,
        to_hash: &str,
        username: &str,
    ) -> String {
        let old_hash = &cl.to_hash[..6];
        let new_hash = &to_hash[..6];
        if cl.from_hash == from_hash {
            format!(
                "{} updated the change_list automatic from {} to {}",
                username, old_hash, new_hash
            )
        } else {
            format!(
                "{} detected upstream changes (base {} → {}). Use Update Branch to sync.",
                username, old_hash, new_hash
            )
        }
    }
}

pub(crate) trait CLRefUpdateVisitor {
    async fn visit(
        &self,
        cl: &mega_cl::Model,
        commit_hash: &str,
        tree_hash: &str,
    ) -> Result<mega_refs::Model, MegaError>;
}

pub(crate) trait CLRefUpdateAcceptor<VT: CLRefUpdateVisitor> {
    async fn accept(
        &self,
        visitor: &VT,
        cl: &mega_cl::Model,
        commit_hash: &str,
        tree_hash: &str,
    ) -> Result<(), MegaError> {
        visitor.visit(cl, commit_hash, tree_hash).await?;
        Ok(())
    }
}

pub(crate) trait TriggerContextBuilder {
    async fn get_context(
        &self,
        cl: &mega_cl::Model,
        username: &str,
    ) -> Result<TriggerContext, MegaError>;
    async fn trigger_build(
        &self,
        storage: Storage,
        git_cache: Arc<GitObjectCache>,
        bellatrix: Arc<Bellatrix>,
        cl: &mega_cl::Model,
        username: &str,
    ) -> Result<Option<i64>, MegaError> {
        BuildTriggerService::build_by_context(
            storage,
            git_cache,
            bellatrix,
            self.get_context(cl, username).await?,
        )
        .await
    }
}

pub(crate) trait Checker {
    async fn check(
        &self,
        storage: Storage,
        username: &str,
        cl: &mega_cl::Model,
    ) -> Result<(), MegaError> {
        let check_reg = CheckerRegistry::new(storage.into(), username.to_string());
        check_reg.run_checks(cl.clone().into()).await?;
        Ok(())
    }
}

pub(crate) trait Director<T: ApiHandler + Clone> {
    async fn get_review_service(&self, storage: &Storage) -> Result<ReviewerService, MegaError>;
    async fn get_api_handler(&self) -> T;
    async fn assign_reviewers(
        &self,
        storage: &Storage,
        cl: &mega_cl::Model,
    ) -> Result<(), MegaError> {
        let handler = self.get_api_handler().await;
        let changed_files = edit_utils::get_changed_files(&handler, cl).await?;
        let policy_contents =
            edit_utils::collect_policy_contents(&handler, cl, &changed_files).await;
        if policy_contents.is_empty() {
            Ok(())
        } else {
            let reviewer_service = self.get_review_service(storage).await?;

            if let Err(e) = reviewer_service
                .assign_system_reviewers(&cl.link, &policy_contents, &changed_files)
                .await
            {
                tracing::warn!("Failed to assign Cedar reviewers: {}", e);
            }

            // Resync reviewers when existing CL updates policy files
            if let Err(e) = reviewer_service
                .sync_system_reviewers(&cl.link, &policy_contents, &changed_files)
                .await
            {
                tracing::warn!("Failed to resync Cedar reviewers: {}", e);
            }
            Ok(())
        }
    }
}

pub(crate) struct CodeEditService<FMT, VT, AC, TCB, CK, HD, DR>
where
    FMT: ConversationMessageFormater,
    VT: CLRefUpdateVisitor,
    AC: CLRefUpdateAcceptor<VT>,
    TCB: TriggerContextBuilder,
    CK: Checker,
    HD: ApiHandler + Clone,
    DR: Director<HD>,
{
    pub repo_path: String,
    pub from_hash: String,
    formator: FMT,
    clref_visitor: VT,
    clref_acceptor: AC,
    builder: TCB,
    checker: CK,
    director: DR,
    // mark HD used
    _marker: std::marker::PhantomData<HD>,
}

pub struct DefaultVisitor<'a> {
    mono_storage: &'a MonoStorage,
    ref_name: &'a str,
}

impl CLRefUpdateVisitor for DefaultVisitor<'_> {
    async fn visit(
        &self,
        _: &mega_cl::Model,
        _: &str,
        _: &str,
    ) -> Result<mega_refs::Model, MegaError> {
        let _ = self.ref_name;
        let _ = self.mono_storage;
        panic!("visitor not implemented!");
    }
}

pub struct DefualtDirector<T: ApiHandler + Clone> {
    pub handler: T,
}

impl<T: ApiHandler + Clone> model::Director<T> for DefualtDirector<T> {
    async fn get_review_service(&self, storage: &Storage) -> Result<ReviewerService, MegaError> {
        Ok(ReviewerService::from_storage(storage.reviewer_storage()))
    }
    async fn get_api_handler(&self) -> T {
        self.handler.clone()
    }
}

impl<
    FMT: ConversationMessageFormater,
    VT: CLRefUpdateVisitor,
    AC: CLRefUpdateAcceptor<VT>,
    TCB: TriggerContextBuilder,
    CK: Checker,
    HD: ApiHandler + Clone,
    DR: Director<HD>,
> CodeEditService<FMT, VT, AC, TCB, CK, HD, DR>
{
    pub fn new(
        repo_path: &str,
        from_hash: &str,
        formator: FMT,
        clref_visitor: VT,
        clref_acceptor: AC,
        builder: TCB,
        checker: CK,
        director: DR,
    ) -> Self {
        Self {
            repo_path: repo_path.to_string(),
            from_hash: from_hash.to_string(),
            formator,
            clref_visitor,
            clref_acceptor,
            builder,
            checker,
            director,
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn update_existing_cl(
        &self,
        cl: mega_cl::Model,
        storage: &Storage,
        from_hash: &str,
        to_hash: &str,
        username: &str,
    ) -> Result<(), MegaError> {
        let cl_stg = storage.cl_storage();
        let comment_stg = storage.conversation_storage();

        let from_same = cl.from_hash == from_hash;
        let to_same = cl.to_hash == to_hash;
        match (from_same, to_same) {
            (true, true) => {
                tracing::info!("repeat commit with change_list: {}, do nothing", cl.id);
            }
            _ => {
                // Freeze cl base for Open cl: do NOT auto-update from_hash here.
                // Only update to_hash to reflect latest edits, and prompt user to run Update Branch.
                comment_stg
                    .add_conversation(
                        &cl.link,
                        username,
                        Some(self.formator.format(&cl, from_hash, to_hash, username)),
                        ConvTypeEnum::Comment,
                    )
                    .await?;
                cl_stg.update_cl_to_hash(cl, to_hash).await?;
            }
        }
        Ok(())
    }

    pub async fn create_new_cl(
        &self,
        storage: &Storage,
        repo_path: &str,
        from_hash: &str,
        to_hash: &str,
        username: &str,
    ) -> Result<mega_cl::Model, MegaError> {
        let cl_link = generate_link();
        let dst_commit = Commit::from_mega_model(
            storage
                .mono_storage()
                .get_commit_by_hash(to_hash)
                .await?
                .expect("invalid to_hash"),
        );
        let cl = storage
            .cl_storage()
            .new_cl_model(
                repo_path,
                &cl_link,
                &dst_commit.format_message(),
                &from_hash,
                to_hash,
                username,
            )
            .await?;

        self.clref_acceptor
            .accept(
                &self.clref_visitor,
                &cl,
                to_hash,
                &dst_commit.tree_id.to_string(),
            )
            .await?;
        self.assign_reviewer(storage, &cl).await?;
        Ok(cl)
    }

    pub async fn update_or_create_cl(
        &self,
        storage: &Storage,
        from_hash: &str,
        to_hash: &str,
        username: &str,
    ) -> Result<mega_cl::Model, MegaError> {
        let path_str = &self.repo_path;
        match storage
            .cl_storage()
            .get_open_cl_by_path(path_str, username)
            .await?
        {
            Some(cl) => {
                self.update_existing_cl(cl.clone(), storage, &cl.from_hash, to_hash, username)
                    .await?;
                Ok(cl)
            }
            None => Ok(self
                .create_new_cl(storage, path_str, from_hash, to_hash, username)
                .await?),
        }
    }

    pub async fn trigger_build(
        &self,
        storage: Storage,
        git_cache: Arc<GitObjectCache>,
        bellatrix: Arc<Bellatrix>,
        cl: &mega_cl::Model,
        username: &str,
    ) -> Result<Option<i64>, MegaError> {
        self.builder
            .trigger_build(storage, git_cache, bellatrix, cl, username)
            .await
    }

    pub async fn trigger_check(
        &self,
        storage: Storage,
        username: &str,
        cl: &mega_cl::Model,
    ) -> Result<(), MegaError> {
        self.checker.check(storage, username, cl).await
    }

    pub async fn assign_reviewer(
        &self,
        storage: &Storage,
        cl: &mega_cl::Model,
    ) -> Result<(), MegaError> {
        self.director.assign_reviewers(storage, cl).await
    }

    pub async fn trigger_build_and_check(
        &self,
        storage: Storage,
        git_cache: Arc<GitObjectCache>,
        bellatrix: Arc<Bellatrix>,
        cl: &mega_cl::Model,
        username: &str,
    ) -> Result<(), MegaError> {
        let _ = self
            .trigger_build(storage.clone(), git_cache, bellatrix, cl, username)
            .await?;
        self.trigger_check(storage, username, cl).await?;
        Ok(())
    }
}
