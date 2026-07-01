use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use common::errors::MegaError;
use git_internal::hash::ObjectHash;

use crate::{application::api_service::mono::ClApplicationService, model::change_list::ClDiffFile};

/// Port for computing CL file diffs used by build trigger handlers.
#[async_trait]
pub trait ChangesPort: Send + Sync {
    async fn get_commit_blobs(
        &self,
        commit_hash: &str,
    ) -> Result<Vec<(PathBuf, ObjectHash)>, MegaError>;

    async fn cl_files_list(
        &self,
        old_files: Vec<(PathBuf, ObjectHash)>,
        new_files: Vec<(PathBuf, ObjectHash)>,
    ) -> Result<Vec<ClDiffFile>, MegaError>;
}

#[async_trait]
impl ChangesPort for ClApplicationService {
    async fn get_commit_blobs(
        &self,
        commit_hash: &str,
    ) -> Result<Vec<(PathBuf, ObjectHash)>, MegaError> {
        self.get_commit_blobs(commit_hash).await
    }

    async fn cl_files_list(
        &self,
        old_files: Vec<(PathBuf, ObjectHash)>,
        new_files: Vec<(PathBuf, ObjectHash)>,
    ) -> Result<Vec<ClDiffFile>, MegaError> {
        self.cl_files_list(old_files, new_files).await
    }
}

#[async_trait]
impl ChangesPort for Arc<dyn ChangesPort> {
    async fn get_commit_blobs(
        &self,
        commit_hash: &str,
    ) -> Result<Vec<(PathBuf, ObjectHash)>, MegaError> {
        self.as_ref().get_commit_blobs(commit_hash).await
    }

    async fn cl_files_list(
        &self,
        old_files: Vec<(PathBuf, ObjectHash)>,
        new_files: Vec<(PathBuf, ObjectHash)>,
    ) -> Result<Vec<ClDiffFile>, MegaError> {
        self.as_ref().cl_files_list(old_files, new_files).await
    }
}

#[async_trait]
impl ChangesPort for Arc<ClApplicationService> {
    async fn get_commit_blobs(
        &self,
        commit_hash: &str,
    ) -> Result<Vec<(PathBuf, ObjectHash)>, MegaError> {
        self.as_ref().get_commit_blobs(commit_hash).await
    }

    async fn cl_files_list(
        &self,
        old_files: Vec<(PathBuf, ObjectHash)>,
        new_files: Vec<(PathBuf, ObjectHash)>,
    ) -> Result<Vec<ClDiffFile>, MegaError> {
        self.as_ref().cl_files_list(old_files, new_files).await
    }
}
