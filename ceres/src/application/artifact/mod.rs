//! Artifact protocol application facade.

use std::{ops::Deref, time::Duration};

use common::errors::MegaError;
use jupiter::{
    service::artifact_service::{ArtifactObjectGcStats, ArtifactService},
    storage::Storage,
};

/// Ceres-facing artifact orchestration service.
#[derive(Clone)]
pub struct ArtifactApplicationService(ArtifactService);

impl ArtifactApplicationService {
    pub fn from_storage(storage: &Storage) -> Self {
        Self(storage.artifact_service.clone())
    }

    pub async fn gc_unreferenced_once(
        &self,
        grace: Duration,
        batch_limit: u64,
    ) -> Result<ArtifactObjectGcStats, MegaError> {
        self.0
            .gc_unreferenced_artifact_objects_once(grace, batch_limit)
            .await
    }
}

impl Deref for ArtifactApplicationService {
    type Target = ArtifactService;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
