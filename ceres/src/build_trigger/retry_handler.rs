use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use common::errors::MegaError;
use jupiter::storage::Storage;

use super::git_push_handler::GitPushHandler;
use crate::{
    api_service::cache::GitObjectCache,
    build_trigger::{
        BuildTrigger, BuildTriggerPayload, BuildTriggerType, TriggerContext, TriggerHandler,
    },
};

/// Handler for retry build triggers.
pub struct RetryHandler {
    git_push_handler: GitPushHandler,
}

impl RetryHandler {
    pub fn new(storage: Storage, git_object_cache: Arc<GitObjectCache>) -> Self {
        Self {
            git_push_handler: GitPushHandler::new(storage, git_object_cache),
        }
    }
}

#[async_trait]
impl TriggerHandler for RetryHandler {
    async fn handle(&self, context: &TriggerContext) -> Result<BuildTrigger, MegaError> {
        // Reuse GitPushHandler logic for getting builds
        let builds = self.git_push_handler.get_builds_for_commit(context).await?;

        let cl_link = context.cl_link.clone().unwrap_or_else(|| {
            format!(
                "retry-{}-{}",
                Utc::now().timestamp_millis(),
                &context.commit_hash[..8.min(context.commit_hash.len())]
            )
        });

        Ok(BuildTrigger {
            trigger_type: BuildTriggerType::Retry,
            trigger_source: context.trigger_source,
            trigger_time: Utc::now(),
            payload: BuildTriggerPayload::Retry(crate::build_trigger::RetryPayload {
                repo: context.repo_path.clone(),
                from_hash: context.from_hash.clone(),
                commit_hash: context.commit_hash.clone(),
                triggered_by: context
                    .triggered_by
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                builds: serde_json::to_value(&builds)
                    .map_err(|e| MegaError::Other(format!("Failed to serialize builds: {}", e)))?,
                original_trigger_id: context.original_trigger_id.ok_or_else(|| {
                    MegaError::Other("Missing original_trigger_id for retry".to_string())
                })?,
                original_cl_link: context.cl_link.clone(),
                cl_link,
                cl_id: context.cl_id,
                ref_name: context.ref_name.clone(),
                ref_type: context.ref_type.clone(),
            }),
        })
    }

    fn trigger_type(&self) -> BuildTriggerType {
        BuildTriggerType::Retry
    }
}
