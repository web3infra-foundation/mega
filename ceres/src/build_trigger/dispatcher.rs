use std::sync::Arc;

use bellatrix::{Bellatrix, orion_client::OrionBuildRequest};
use common::errors::MegaError;
use jupiter::storage::Storage;

use crate::build_trigger::{BuildTrigger, BuildTriggerPayload, SerializableBuildInfo};

/// Handles dispatching build triggers to the build execution layer (Bellatrix/Orion).
pub struct BuildDispatcher {
    storage: Storage,
    bellatrix: Arc<Bellatrix>,
}

impl BuildDispatcher {
    pub fn new(storage: Storage, bellatrix: Arc<Bellatrix>) -> Self {
        Self { storage, bellatrix }
    }

    /// Dispatch a build trigger.
    ///
    /// This method:
    /// 1. Persists the trigger to the database
    /// 2. Sends the build request to Bellatrix/Orion asynchronously
    ///
    /// Returns the ID of the created trigger record.
    pub async fn dispatch(&self, trigger: BuildTrigger) -> Result<i64, MegaError> {
        let trigger_payload = serde_json::to_value(&trigger.payload).map_err(|e| {
            tracing::error!("Failed to serialize payload: {}", e);
            MegaError::Other(format!("Failed to serialize payload: {}", e))
        })?;

        let db_record = self
            .storage
            .build_trigger_storage()
            .insert(
                trigger.trigger_type.to_string(),
                trigger.trigger_source.to_string(),
                trigger_payload,
            )
            .await?;

        // Persist to database
        tracing::info!("BuildDispatcher: Persisted trigger ID: {}", db_record.id);

        if !self.bellatrix.enable_build() {
            tracing::info!("BuildDispatcher: Completed (build system disabled)");
            return Ok(db_record.id);
        }

        // Extract data from payload using pattern matching
        let (cl_link, repo, builds_json, cl_id) = match &trigger.payload {
            BuildTriggerPayload::GitPush(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
            BuildTriggerPayload::Manual(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
            BuildTriggerPayload::Retry(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
            BuildTriggerPayload::Webhook(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
            BuildTriggerPayload::Schedule(p) => (&p.cl_link, &p.repo, &p.builds, p.cl_id),
        };

        let builds: Vec<SerializableBuildInfo> = serde_json::from_value(builds_json.clone())
            .map_err(|e| {
                tracing::error!("Failed to deserialize builds from payload: {}", e);
                MegaError::Other(format!("Failed to deserialize builds from payload: {}", e))
            })?;

        let bellatrix_builds: Vec<bellatrix::orion_client::BuildInfo> = builds
            .into_iter()
            .enumerate()
            .map(|(idx, info)| {
                tracing::debug!("  Build [{}]: {} change(s)", idx + 1, info.changes.len());
                bellatrix::orion_client::BuildInfo {
                    changes: info.changes.into_iter().map(|s| s.into()).collect(),
                }
            })
            .collect();

        let req = OrionBuildRequest {
            cl_link: cl_link.to_string(),
            mount_path: repo.to_string(),
            cl: cl_id.unwrap_or(0),
            builds: bellatrix_builds,
        };

        // Dispatch asynchronously
        let bellatrix = self.bellatrix.clone();
        let trigger_id = db_record.id;
        tokio::spawn(async move {
            match bellatrix.on_post_receive(req).await {
                Ok(_) => {
                    tracing::info!(
                        "BuildDispatcher: Build request sent to Bellatrix (Trigger ID: {})",
                        trigger_id
                    );
                }
                Err(err) => {
                    tracing::error!(
                        "BuildDispatcher: Failed to dispatch build (Trigger ID: {}): {}",
                        trigger_id,
                        err
                    );
                }
            }
        });

        Ok(db_record.id)
    }
}
