use std::sync::Arc;

use api_model::buck2::{api::TaskBuildRequest, status::Status, types::ProjectRelativePath};
use bellatrix::Bellatrix;
use common::errors::MegaError;
use jupiter::storage::Storage;

use crate::build_trigger::{BuildTrigger, BuildTriggerPayload};

/// Handles dispatching build triggers to the build execution layer (Bellatrix/Orion).
pub struct BuildDispatcher {
    storage: Storage,
    bellatrix: Arc<Bellatrix>,
}

impl BuildDispatcher {
    pub fn new(storage: Storage, bellatrix: Arc<Bellatrix>) -> Self {
        Self { storage, bellatrix }
    }

    pub async fn dispatch(&self, trigger: BuildTrigger) -> Result<i64, MegaError> {
        let trigger_payload = serde_json::to_value(&trigger.payload).map_err(|e| {
            tracing::error!("Failed to serialize payload: {}", e);
            MegaError::Other(format!("Failed to serialize payload: {}", e))
        })?;

        // Determine task_id based on whether build system is enabled
        let task_id: Option<uuid::Uuid> = if self.bellatrix.enable_build() {
            // Extract data from payload using pattern matching
            let (cl_link, repo, builds_json, cl_id) = match &trigger.payload {
                BuildTriggerPayload::GitPush(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
                BuildTriggerPayload::Manual(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
                BuildTriggerPayload::Retry(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
                BuildTriggerPayload::Webhook(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
                BuildTriggerPayload::Schedule(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
                BuildTriggerPayload::WebEdit(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
                BuildTriggerPayload::BuckFileUpload(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
            };

            let changes: Vec<Status<ProjectRelativePath>> =
                serde_json::from_value(builds_json.clone()).map_err(|e| {
                    tracing::error!("Failed to deserialize builds from payload: {}", e);
                    MegaError::Other(format!("Failed to deserialize builds from payload: {}", e))
                })?;

            let req = TaskBuildRequest {
                repo: repo.to_string(),
                cl_link: cl_link.to_string(),
                cl_id: cl_id.unwrap_or(0),
                changes,
                targets: None,
            };

            let task_id_str = self.bellatrix.on_post_receive(req).await.map_err(|e| {
                tracing::error!("Failed to dispatch build to Bellatrix: {}", e);
                MegaError::Other(format!("Failed to dispatch build to Bellatrix: {}", e))
            })?;

            let task_uuid = uuid::Uuid::parse_str(&task_id_str).map_err(|e| {
                tracing::error!("Invalid task_id format '{}': {}", task_id_str, e);
                MegaError::Other(format!("Invalid task_id format '{}': {}", task_id_str, e))
            })?;

            Some(task_uuid)
        } else {
            tracing::info!("BuildDispatcher: Build system disabled, skipping Orion call");
            None
        };

        // Insert trigger record with task_id (complete record in one operation)
        let db_record = self
            .storage
            .build_trigger_storage()
            .insert(
                trigger.trigger_type.to_string(),
                trigger.trigger_source.to_string(),
                trigger_payload,
                task_id,
            )
            .await?;

        tracing::info!(
            "BuildDispatcher: Trigger persisted (ID: {}, Task ID: {:?})",
            db_record.id,
            task_id
        );

        Ok(db_record.id)
    }
}
