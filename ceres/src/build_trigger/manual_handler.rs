use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use common::errors::MegaError;
use jupiter::storage::Storage;

use super::changes_calculator::ChangesCalculator;
use crate::{
    api_service::cache::GitObjectCache,
    build_trigger::{
        BuildTrigger, BuildTriggerPayload, BuildTriggerType, ManualPayload, TriggerContext,
        TriggerHandler,
    },
};

/// Handler for manual build triggers.
pub struct ManualHandler {
    storage: Storage,
    changes_calculator: ChangesCalculator,
}

impl ManualHandler {
    pub fn new(storage: Storage, git_object_cache: Arc<GitObjectCache>) -> Self {
        Self {
            storage: storage.clone(),
            changes_calculator: ChangesCalculator::new(storage, git_object_cache),
        }
    }

    /// Get the parent commit hash for a given commit.
    /// Returns the first parent if available, otherwise returns the same commit hash.
    async fn get_parent_commit(&self, commit_hash: &str) -> Result<String, MegaError> {
        let commit = self
            .storage
            .mono_storage()
            .get_commit_by_hash(commit_hash)
            .await?
            .ok_or_else(|| {
                MegaError::Other(format!("[code:404] Commit not found: {}", commit_hash))
            })?;

        // Parse parents_id JSON array
        let parent_ids: Vec<String> =
            serde_json::from_value(commit.parents_id.clone()).unwrap_or_default();

        // Use first parent if available, otherwise return same hash (initial commit case)
        Ok(parent_ids
            .first()
            .cloned()
            .unwrap_or_else(|| commit_hash.to_string()))
    }
}

#[async_trait]
impl TriggerHandler for ManualHandler {
    async fn handle(&self, context: &TriggerContext) -> Result<BuildTrigger, MegaError> {
        let from_hash = if context.from_hash == context.commit_hash {
            self.get_parent_commit(&context.commit_hash).await?
        } else {
            context.from_hash.clone()
        };

        let adjusted_context = TriggerContext {
            from_hash: from_hash.clone(),
            ..context.clone()
        };

        let builds = self
            .changes_calculator
            .get_builds_for_commit(&adjusted_context)
            .await?;

        let cl_link = context.cl_link.clone().unwrap_or_else(|| {
            format!(
                "manual-{}-{}",
                Utc::now().timestamp_millis(),
                &context.commit_hash[..8.min(context.commit_hash.len())]
            )
        });

        Ok(BuildTrigger {
            trigger_type: BuildTriggerType::Manual,
            trigger_source: context.trigger_source,
            trigger_time: Utc::now(),
            payload: BuildTriggerPayload::Manual(ManualPayload {
                repo: context.repo_path.clone(),
                commit_hash: context.commit_hash.clone(),
                triggered_by: context
                    .triggered_by
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                builds: serde_json::to_value(&builds)
                    .map_err(|e| MegaError::Other(format!("Failed to serialize builds: {}", e)))?,
                params: context.params.clone(),
                cl_link: cl_link.clone(),
                cl_id: context.cl_id,
                ref_name: context.ref_name.clone(),
                ref_type: context.ref_type.clone(),
            }),
        })
    }

    fn trigger_type(&self) -> BuildTriggerType {
        BuildTriggerType::Manual
    }
}
