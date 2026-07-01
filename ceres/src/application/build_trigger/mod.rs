use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use common::errors::MegaError;
use jupiter::storage::Storage;

mod buck_upload_handler;
mod changes_calculator;
mod changes_port;
mod dispatcher;
mod git_push_handler;
mod manual_handler;
mod model;
mod port;
mod ref_resolver;
mod retry_handler;
mod web_edit_handler;

// Export all models from the single model file
pub use changes_port::ChangesPort;
pub use model::*;
pub use port::{BuildDispatchPort, SharedBuildDispatch};
pub use ref_resolver::{RefResolver, RefType, ResolvedRef};
pub mod service;
use buck_upload_handler::BuckFileUploadHandler;
use dispatcher::BuildDispatcher;
use git_push_handler::GitPushHandler;
use manual_handler::ManualHandler;
use retry_handler::RetryHandler;
pub use service::BuildTriggerService;
use web_edit_handler::WebEditHandler;

/// Trait for handling different types of build triggers.
#[async_trait]
pub trait TriggerHandler: Send + Sync {
    /// Handle the trigger and return a BuildTrigger.
    async fn handle(&self, context: &TriggerContext) -> Result<BuildTrigger, MegaError>;

    /// Get the trigger type this handler supports.
    fn trigger_type(&self) -> BuildTriggerType;
}

/// Registry for managing and dispatching build triggers.
pub struct TriggerRegistry {
    handlers: HashMap<BuildTriggerType, Box<dyn TriggerHandler>>,
    dispatcher: Arc<BuildDispatcher>,
}

impl TriggerRegistry {
    /// Create a new TriggerRegistry with all handlers registered.
    pub fn new(
        storage: Storage,
        build_dispatch: SharedBuildDispatch,
        changes_port: Arc<dyn ChangesPort>,
    ) -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
            dispatcher: Arc::new(BuildDispatcher::new(
                storage.clone(),
                build_dispatch.clone(),
            )),
        };

        registry.register(Box::new(GitPushHandler::new(changes_port.clone())));
        registry.register(Box::new(ManualHandler::new(
            storage.clone(),
            changes_port.clone(),
        )));
        registry.register(Box::new(RetryHandler::new(changes_port.clone())));
        registry.register(Box::new(WebEditHandler::new(changes_port.clone())));
        registry.register(Box::new(BuckFileUploadHandler::new(changes_port)));

        registry
    }

    /// Register a trigger handler.
    fn register(&mut self, handler: Box<dyn TriggerHandler>) {
        self.handlers.insert(handler.trigger_type(), handler);
    }

    /// Trigger a build using the unified interface.
    ///
    /// This is the single entry point for all build triggers.
    ///
    /// Returns the ID of the created trigger record.
    pub async fn trigger_build(&self, context: TriggerContext) -> Result<i64, MegaError> {
        tracing::info!(
            "TriggerRegistry: Received {:?} build trigger for {} (commit: {})",
            context.trigger_type,
            context.repo_path,
            &context.commit_hash[..8.min(context.commit_hash.len())]
        );

        let handler = self.handlers.get(&context.trigger_type).ok_or_else(|| {
            MegaError::Other(format!(
                "No handler for trigger type: {:?}",
                context.trigger_type
            ))
        })?;

        let trigger = handler.handle(&context).await?;

        let trigger_id = self.dispatcher.dispatch(trigger).await?;

        tracing::info!(
            "TriggerRegistry: Build trigger completed (ID: {})",
            trigger_id
        );

        Ok(trigger_id)
    }
}
