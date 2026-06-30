use std::path::PathBuf;

use async_trait::async_trait;
use common::errors::MegaError;
use git_internal::hash::ObjectHash;

use crate::{application::api_service::mono::MonoApiService, model::change_list::ClDiffFile};

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
impl ChangesPort for MonoApiService {
    async fn get_commit_blobs(
        &self,
        commit_hash: &str,
    ) -> Result<Vec<(PathBuf, ObjectHash)>, MegaError> {
        MonoApiService::get_commit_blobs(self, commit_hash).await
    }

    async fn cl_files_list(
        &self,
        old_files: Vec<(PathBuf, ObjectHash)>,
        new_files: Vec<(PathBuf, ObjectHash)>,
    ) -> Result<Vec<ClDiffFile>, MegaError> {
        MonoApiService::cl_files_list(self, old_files, new_files).await
    }
}
