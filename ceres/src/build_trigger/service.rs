use std::sync::Arc;

use api_model::common::Pagination;
use bellatrix::Bellatrix;
use common::errors::MegaError;
use jupiter::storage::Storage;

use crate::{
    api_service::cache::GitObjectCache,
    build_trigger::{
        RefResolver, TriggerRegistry,
        model::{
            BuildParams, GitPushEvent, ListTriggersParams, TriggerContext, TriggerRecord,
            TriggerResponse,
        },
    },
};

/// Service for handling build trigger operations
pub struct BuildTriggerService {
    storage: Storage,
    registry: TriggerRegistry,
    bellatrix: Arc<Bellatrix>,
}

impl BuildTriggerService {
    pub fn new(
        storage: Storage,
        git_object_cache: Arc<GitObjectCache>,
        bellatrix: Arc<Bellatrix>,
    ) -> Self {
        let registry = TriggerRegistry::new(storage.clone(), git_object_cache, bellatrix.clone());
        Self {
            storage,
            registry,
            bellatrix,
        }
    }

    fn check_build_enabled(&self) -> Result<(), MegaError> {
        if !self.bellatrix.enable_build() {
            return Err(MegaError::Other(
                "[code:503] Build system is not enabled".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn handle_git_push_event(
        storage: Storage,
        git_cache: Arc<GitObjectCache>,
        bellatrix: Arc<Bellatrix>,
        event: GitPushEvent,
    ) -> Result<Option<i64>, MegaError> {
        if !bellatrix.enable_build() {
            return Ok(None);
        }
        let registry = TriggerRegistry::new(storage, git_cache, bellatrix);

        let context = TriggerContext::from_git_push(
            event.repo_path,
            event.from_hash,
            event.commit_hash,
            event.cl_link,
            event.cl_id,
            event.triggered_by,
        );

        let id = registry.trigger_build(context).await?;
        Ok(Some(id))
    }

    pub async fn build_by_context(
        storage: Storage,
        git_cache: Arc<GitObjectCache>,
        bellatrix: Arc<Bellatrix>,
        context: TriggerContext,
    ) -> Result<Option<i64>, MegaError> {
        if !bellatrix.enable_build() {
            return Ok(None);
        }
        let registry = TriggerRegistry::new(storage, git_cache, bellatrix);

        let id = registry.trigger_build(context).await?;
        Ok(Some(id))
    }

    /// Create a manual trigger (complete workflow)
    pub async fn create_manual_trigger(
        &self,
        repo_path: String,
        ref_name: Option<String>,
        params: Option<BuildParams>,
        triggered_by: String,
    ) -> Result<TriggerResponse, MegaError> {
        self.check_build_enabled()?;

        // 1. Resolve reference
        let ref_resolver = RefResolver::new(self.storage.clone());
        let resolved = ref_resolver
            .resolve(ref_name.as_deref())
            .await
            .map_err(|_| {
                let ref_str = ref_name.unwrap_or_else(|| "main".to_string());
                MegaError::Other(format!("[code:404] Reference not found: {}", ref_str))
            })?;

        // 2. Build context
        let mut context = TriggerContext::from_manual(
            repo_path,
            resolved.commit_hash.clone(),
            triggered_by,
            params,
        );
        context.ref_name = Some(resolved.ref_name.clone());
        context.ref_type = Some(resolved.ref_type.as_str().to_string());

        // 3. Create trigger
        let trigger_id = self.registry.trigger_build(context).await?;

        // 4. Get and return response
        self.get_trigger_response(trigger_id).await
    }

    /// Get trigger details
    pub async fn get_trigger(&self, trigger_id: i64) -> Result<TriggerResponse, MegaError> {
        self.check_build_enabled()?;
        self.get_trigger_response(trigger_id).await
    }

    /// List triggers with filters and pagination
    pub async fn list_triggers(
        &self,
        params: ListTriggersParams,
        pagination: Pagination,
    ) -> Result<(Vec<TriggerResponse>, i64), MegaError> {
        self.check_build_enabled()?;

        let (triggers, total) = self
            .storage
            .build_trigger_storage()
            .get_trigger_list(params, pagination)
            .await?;

        let responses = triggers
            .into_iter()
            .map(|t| {
                let record = TriggerRecord::from_db_model(t);
                TriggerResponse::from_trigger_record(&record)
                    .map_err(|e| MegaError::Other(format!("Failed to parse trigger: {}", e)))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok((responses, total as i64))
    }

    /// Retry a previous build trigger
    pub async fn retry_trigger(
        &self,
        original_trigger_id: i64,
        triggered_by: String,
    ) -> Result<TriggerResponse, MegaError> {
        self.check_build_enabled()?;

        // 1. Get original trigger
        let original_trigger = self
            .storage
            .build_trigger_storage()
            .get_by_id(original_trigger_id)
            .await?
            .ok_or_else(|| {
                MegaError::Other(format!(
                    "[code:404] Trigger not found: {}",
                    original_trigger_id
                ))
            })?;

        // 2. Parse payload
        let trigger_record = TriggerRecord::from_db_model(original_trigger);
        let payload = trigger_record
            .parse_payload()
            .map_err(|e| MegaError::Other(format!("Failed to parse payload: {}", e)))?;

        // 3. Construct retry context
        let context = TriggerContext::from_retry(
            payload.repo_path().to_string(),
            payload.from_hash().to_string(),
            payload.commit_hash().to_string(),
            Some(payload.cl_link().to_string()),
            payload.cl_id(),
            Some(triggered_by),
            original_trigger_id,
        );

        // 4. Create new trigger
        let new_trigger_id = self.registry.trigger_build(context).await?;

        // 5. Get and return response
        self.get_trigger_response(new_trigger_id).await
    }

    /// Internal helper method: get trigger response
    async fn get_trigger_response(&self, trigger_id: i64) -> Result<TriggerResponse, MegaError> {
        let model = self
            .storage
            .build_trigger_storage()
            .get_by_id(trigger_id)
            .await?
            .ok_or_else(|| {
                MegaError::Other(format!("[code:404] Trigger not found: {}", trigger_id))
            })?;

        let record = TriggerRecord::from_db_model(model);
        TriggerResponse::from_trigger_record(&record)
            .map_err(|e| MegaError::Other(format!("Failed to parse trigger: {}", e)))
    }
}
