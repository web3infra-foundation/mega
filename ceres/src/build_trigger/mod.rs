use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use bellatrix::Bellatrix;
use common::errors::MegaError;
use jupiter::storage::Storage;

use crate::api_service::cache::GitObjectCache;

mod changes_calculator;
mod dispatcher;
mod git_push_handler;
mod manual_handler;
mod model;
mod ref_resolver;
mod retry_handler;

// Export all models from the single model file
pub use model::*;
pub use ref_resolver::{RefResolver, RefType, ResolvedRef};
pub mod service;
use dispatcher::BuildDispatcher;
use git_push_handler::GitPushHandler;
use manual_handler::ManualHandler;
use retry_handler::RetryHandler;
pub use service::BuildTriggerService;

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
        git_object_cache: Arc<GitObjectCache>,
        bellatrix: Arc<Bellatrix>,
    ) -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
            dispatcher: Arc::new(BuildDispatcher::new(storage.clone(), bellatrix)),
        };

        // Register core handlers (Git Push, Manual, Retry)
        registry.register(Box::new(GitPushHandler::new(
            storage.clone(),
            git_object_cache.clone(),
        )));
        registry.register(Box::new(ManualHandler::new(
            storage.clone(),
            git_object_cache.clone(),
        )));
        registry.register(Box::new(RetryHandler::new(
            storage.clone(),
            git_object_cache.clone(),
        )));

        // Note: Webhook and Schedule handlers are reserved for future implementation
        // but not registered yet as they are not part of the current requirements

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

        // Find the appropriate handler
        let handler = self.handlers.get(&context.trigger_type).ok_or_else(|| {
            MegaError::Other(format!(
                "No handler for trigger type: {:?}",
                context.trigger_type
            ))
        })?;

        // Handle the trigger
        let trigger = handler.handle(&context).await?;

        // Dispatch the build and return the trigger ID
        let trigger_id = self.dispatcher.dispatch(trigger).await?;

        tracing::info!(
            "TriggerRegistry: Build trigger completed (ID: {})",
            trigger_id
        );

        Ok(trigger_id)
    }
}
