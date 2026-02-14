use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub use api_model::buck2::{status::Status, types::ProjectRelativePath};
use common::errors::MegaError;
use git_internal::hash::ObjectHash;
use jupiter::storage::Storage;

use crate::{
    api_service::{cache::GitObjectCache, mono_api_service::MonoApiService},
    build_trigger::TriggerContext,
    model::change_list::{ClDiffFile, ClFilesRes},
};

pub struct ChangesCalculator {
    storage: Storage,
    git_object_cache: Arc<GitObjectCache>,
}

impl ChangesCalculator {
    pub fn new(storage: Storage, git_object_cache: Arc<GitObjectCache>) -> Self {
        Self {
            storage,
            git_object_cache,
        }
    }

    pub async fn get_builds_for_commit(
        &self,
        context: &TriggerContext,
    ) -> Result<Vec<Status<ProjectRelativePath>>, MegaError> {
        let old_files = self.get_commit_blobs(&context.from_hash).await?;
        let new_files = self.get_commit_blobs(&context.commit_hash).await?;
        let diff_files = self.cl_files_list(old_files, new_files).await?;

        let changes = self.build_changes(&context.repo_path, diff_files)?;

        Ok(changes)
    }

    fn build_changes(
        &self,
        cl_path: &str,
        cl_diff_files: Vec<ClDiffFile>,
    ) -> Result<Vec<Status<ProjectRelativePath>>, MegaError> {
        let cl_base = PathBuf::from(cl_path);
        let path_str = cl_base.to_str().ok_or_else(|| {
            MegaError::Other(format!("CL base path is not valid UTF-8: {:?}", cl_base))
        })?;

        let changes = cl_diff_files
            .into_iter()
            .map(|m| {
                let mut item: ClFilesRes = m.into();
                item.path = cl_base.join(item.path).to_string_lossy().to_string();
                item
            })
            .collect::<Vec<_>>();

        let counter_changes = changes
            .iter()
            .filter(|&s| PathBuf::from(&s.path).starts_with(&cl_base))
            .map(|s| {
                let rel = Path::new(&s.path)
                    .strip_prefix(path_str)
                    .map_err(|_| {
                        MegaError::Other(format!("Invalid project-relative path: {}", s.path))
                    })?
                    .to_string_lossy()
                    .replace('\\', "/")
                    .trim_start_matches('/')
                    .to_string();

                let status = if s.action == "new" {
                    Status::Added(ProjectRelativePath::new(&rel))
                } else if s.action == "deleted" {
                    Status::Removed(ProjectRelativePath::new(&rel))
                } else if s.action == "modified" {
                    Status::Modified(ProjectRelativePath::new(&rel))
                } else {
                    return Err(MegaError::Other(format!(
                        "Unsupported change action: {}",
                        s.action
                    )));
                };
                Ok(status)
            })
            .collect::<Result<Vec<_>, MegaError>>()?;

        Ok(counter_changes)
    }

    async fn get_commit_blobs(
        &self,
        commit_hash: &str,
    ) -> Result<Vec<(PathBuf, ObjectHash)>, MegaError> {
        let api_service = MonoApiService {
            storage: self.storage.clone(),
            git_object_cache: self.git_object_cache.clone(),
        };
        api_service.get_commit_blobs(commit_hash).await
    }

    async fn cl_files_list(
        &self,
        old_files: Vec<(PathBuf, ObjectHash)>,
        new_files: Vec<(PathBuf, ObjectHash)>,
    ) -> Result<Vec<ClDiffFile>, MegaError> {
        let api_service = MonoApiService {
            storage: self.storage.clone(),
            git_object_cache: self.git_object_cache.clone(),
        };
        api_service.cl_files_list(old_files, new_files).await
    }
}
