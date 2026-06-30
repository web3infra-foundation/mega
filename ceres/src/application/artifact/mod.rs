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

    pub fn weak_etag_for_oid_size(oid: &str, size_bytes: i64) -> String {
        ArtifactService::weak_etag_for_oid_size(oid, size_bytes)
    }

    pub fn parse_artifact_object_range(
        range_header_value: Option<&str>,
        len: u64,
    ) -> Result<Option<(u64, u64)>, MegaError> {
        ArtifactService::parse_artifact_object_range(range_header_value, len)
    }
}

impl Deref for ArtifactApplicationService {
    type Target = ArtifactService;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
