use callisto::{mega_cl, mega_refs};
use common::{
    errors::MegaError,
    utils::{self},
};
use git_internal::errors::GitError;
use jupiter::storage::Storage;

use crate::{
    api_service::mono_api_service::MonoApiService, build_trigger::TriggerContext, code_edit::model,
    model::git::EditCLMode,
};

pub struct OneditFormator;
impl model::ConversationMessageFormater for OneditFormator {
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
                "{} edited the change_list automatic from {} to {}.",
                username, old_hash, new_hash
            )
        }
    }
}

pub struct OneditVisitor {}
impl model::CLRefUpdateVisitor for OneditVisitor {
    async fn visit(
        &self,
        cl: &mega_cl::Model,
        commit_hash: &str,
        tree_hash: &str,
    ) -> Result<mega_refs::Model, MegaError> {
        Ok(mega_refs::Model::new(
            &cl.path,
            utils::cl_ref_name(&cl.link),
            commit_hash.to_string(),
            tree_hash.to_string(),
            true,
        ))
    }
}

pub struct OneditAcceptor {}

impl<VT: model::CLRefUpdateVisitor> model::CLRefUpdateAcceptor<VT> for OneditAcceptor {
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

pub struct OneditTrigerBuilder {}

impl model::TriggerContextBuilder for OneditTrigerBuilder {
    async fn get_context(
        &self,
        cl: &mega_cl::Model,
        username: &str,
    ) -> Result<TriggerContext, MegaError> {
        Ok(TriggerContext::from_git_push(
            cl.path.clone(),
            cl.from_hash.clone(),
            cl.to_hash.clone(),
            cl.link.clone(),
            Some(cl.id),
            Some(username.to_string()),
        ))
    }
}

pub struct OneditChecker {}

impl model::Checker for OneditChecker {}

pub(crate) type OneditCodeEdit = model::CodeEditService<
    OneditFormator,
    OneditVisitor,
    OneditAcceptor,
    OneditTrigerBuilder,
    OneditChecker,
    MonoApiService,
    model::DefualtDirector<MonoApiService>,
>;

impl OneditCodeEdit {
    pub fn from(repo_path: &str, from_hash: &str, handler: &MonoApiService) -> Self {
        Self::new(
            repo_path,
            from_hash,
            OneditFormator {},
            OneditVisitor {},
            OneditAcceptor {},
            OneditTrigerBuilder {},
            OneditChecker {},
            model::DefualtDirector::<MonoApiService> {
                handler: handler.clone(),
            },
        )
    }

    pub async fn find_or_create_cl_for_edit(
        &self,
        storage: &Storage,
        editor: &OneditCodeEdit,
        mode: EditCLMode,
        to_hash: &str,
        username: &str,
    ) -> Result<mega_cl::Model, GitError> {
        let repo_path = &self.repo_path;
        match mode {
            EditCLMode::ForceCreate => Ok(editor
                .create_new_cl(storage, repo_path, &self.from_hash, to_hash, username)
                .await?),
            EditCLMode::TryReuse(None) => {
                if let Some(existing_cl) = storage
                    .cl_storage()
                    .get_open_cl_by_path(repo_path, username)
                    .await
                    .map_err(|e| GitError::CustomError(format!("Failed to fetch CL: {}", e)))?
                {
                    editor
                        .update_existing_cl(
                            existing_cl.clone(),
                            storage,
                            &existing_cl.from_hash,
                            to_hash,
                            username,
                        )
                        .await?;
                    Ok(existing_cl)
                } else {
                    Ok(editor
                        .create_new_cl(storage, repo_path, &self.from_hash, to_hash, username)
                        .await?)
                }
            }
            EditCLMode::TryReuse(Some(link)) => match storage.cl_storage().get_cl(&link).await {
                Ok(Some(existing_cl)) => {
                    editor
                        .update_existing_cl(
                            existing_cl.clone(),
                            storage,
                            &existing_cl.from_hash,
                            to_hash,
                            username,
                        )
                        .await?;
                    Ok(existing_cl)
                }
                _ => Err(GitError::CustomError(format!("link {} not found", link))),
            },
        }
    }
}
