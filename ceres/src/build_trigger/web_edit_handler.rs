use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use common::errors::MegaError;
use jupiter::storage::Storage;

use super::changes_calculator::ChangesCalculator;
use crate::{
    api_service::cache::GitObjectCache,
    build_trigger::{
        BuildTrigger, BuildTriggerPayload, BuildTriggerType, TriggerContext, TriggerHandler,
        WebEditPayload,
    },
};

/// Handler for web edit trigger
pub struct WebEditHandler {
    changes_calculator: ChangesCalculator,
}

impl WebEditHandler {
    pub fn new(storage: Storage, git_object_cache: Arc<GitObjectCache>) -> Self {
        Self {
            changes_calculator: ChangesCalculator::new(storage, git_object_cache),
        }
    }
}

#[async_trait]
impl TriggerHandler for WebEditHandler {
    async fn handle(&self, context: &TriggerContext) -> Result<BuildTrigger, MegaError> {
        let builds = self
            .changes_calculator
            .get_builds_for_commit(context)
            .await?;

        let cl_link = context.cl_link.clone().unwrap_or_else(|| {
            format!(
                "webedit-{}-{}",
                Utc::now().timestamp_millis(),
                &context.commit_hash[..8.min(context.commit_hash.len())]
            )
        });

        Ok(BuildTrigger {
            trigger_type: context.trigger_type,
            trigger_source: context.trigger_source,
            trigger_time: Utc::now(),
            payload: BuildTriggerPayload::WebEdit(WebEditPayload {
                repo: context.repo_path.clone(),
                from_hash: context.from_hash.clone(),
                commit_hash: context.commit_hash.clone(),
                cl_link,
                cl_id: context.cl_id,
                builds: serde_json::to_value(&builds)
                    .map_err(|e| MegaError::Other(format!("Failed to serialize builds: {}", e)))?,
                triggered_by: context.triggered_by.clone(),
            }),
        })
    }

    fn trigger_type(&self) -> BuildTriggerType {
        BuildTriggerType::WebEdit
    }
}
