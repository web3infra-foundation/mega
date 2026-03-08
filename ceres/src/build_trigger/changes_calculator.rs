use std::{path::PathBuf, sync::Arc};

pub use api_model::buck2::{status::Status, types::ProjectRelativePath};
use common::errors::MegaError;
use git_internal::hash::ObjectHash;
use jupiter::storage::Storage;

use crate::{
    api_service::{cache::GitObjectCache, mono_api_service::MonoApiService},
    build_trigger::TriggerContext,
    model::change_list::ClDiffFile,
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

        let changes = self.build_changes(diff_files)?;

        Ok(changes)
    }

    fn build_changes(
        &self,
        cl_diff_files: Vec<ClDiffFile>,
    ) -> Result<Vec<Status<ProjectRelativePath>>, MegaError> {
        let to_project_relative = |path: &PathBuf| -> Result<ProjectRelativePath, MegaError> {
            let rel = path
                .to_string_lossy()
                .replace('\\', "/")
                .trim_start_matches('/')
                .to_string();
            Ok(ProjectRelativePath::new(&rel))
        };

        let mut counter_changes = Vec::new();
        for change in cl_diff_files {
            match change {
                ClDiffFile::New(path, _) => {
                    counter_changes.push(Status::Added(to_project_relative(&path)?));
                }
                ClDiffFile::Deleted(path, _) => {
                    counter_changes.push(Status::Removed(to_project_relative(&path)?));
                }
                ClDiffFile::Modified(path, _, _) => {
                    counter_changes.push(Status::Modified(to_project_relative(&path)?));
                }
                ClDiffFile::Renamed(old_path, new_path, _, _, _)
                | ClDiffFile::Moved(old_path, new_path, _, _, _) => {
                    counter_changes.push(Status::Removed(to_project_relative(&old_path)?));
                    counter_changes.push(Status::Added(to_project_relative(&new_path)?));
                }
            }
        }

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
