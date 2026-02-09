use callisto::{mega_cl, mega_refs};
use common::errors::MegaError;

use crate::{
    api_service::mono_api_service::MonoApiService,
    build_trigger::TriggerContext,
    code_edit::model::{self, CLRefUpdateVisitor},
};

pub struct OnpushFormator;
impl model::ConversationMessageFormater for OnpushFormator {}

pub struct OnpushVisitor {}
impl model::CLRefUpdateVisitor for OnpushVisitor {
    async fn visit(
        &self,
        _: &mega_cl::Model,
        _: &str,
        _: &str,
    ) -> Result<mega_refs::Model, MegaError> {
        panic!("visit not implemented");
    }
}

pub struct OnpushAcceptor {}

impl<VT: CLRefUpdateVisitor> model::CLRefUpdateAcceptor<VT> for OnpushAcceptor {
    async fn accept(&self, _: &VT, _: &mega_cl::Model, _: &str, _: &str) -> Result<(), MegaError> {
        Ok(())
    }
}

pub struct OnpushTrigerBuilder {}

impl model::TriggerContextBuilder for OnpushTrigerBuilder {
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

pub struct OnpushChecker {}

impl model::Checker for OnpushChecker {}

pub(crate) type OnpushCodeEdit = model::CodeEditService<
    OnpushFormator,
    OnpushVisitor,
    OnpushAcceptor,
    OnpushTrigerBuilder,
    OnpushChecker,
    MonoApiService,
    model::DefualtDirector<MonoApiService>,
>;

// impl<'a> model::CodeEditService<OnpushFormator, OnpushVisitor, OnpushAcceptor, OnpushTrigerBuilder, OnpushChecker, model::DefualthDirector<'a, MonoRepo>> {
impl OnpushCodeEdit {
    pub fn from(repo_path: &str, from_hash: &str, handler: &MonoApiService) -> Self {
        Self::new(
            repo_path,
            from_hash,
            OnpushFormator {},
            OnpushVisitor {},
            OnpushAcceptor {},
            OnpushTrigerBuilder {},
            OnpushChecker {},
            model::DefualtDirector {
                handler: handler.clone(),
            },
        )
    }
}
