use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use bellatrix::orion_client::{BuildInfo, ProjectRelativePath, Status};
use chrono::Utc;
use common::errors::MegaError;
use git_internal::hash::ObjectHash;
use jupiter::storage::Storage;

use crate::{
    api_service::{cache::GitObjectCache, mono_api_service::MonoApiService},
    build_trigger::{
        BuildTrigger, BuildTriggerPayload, BuildTriggerType, TriggerContext, TriggerHandler,
    },
    model::change_list::ClFilesRes,
};

/// Handler for Git push triggers.
pub struct GitPushHandler {
    storage: Storage,
    git_object_cache: Arc<GitObjectCache>,
}

impl GitPushHandler {
    pub fn new(storage: Storage, git_object_cache: Arc<GitObjectCache>) -> Self {
        Self {
            storage,
            git_object_cache,
        }
    }

    pub async fn get_builds_for_commit(
        &self,
        context: &TriggerContext,
    ) -> Result<Vec<BuildInfo>, MegaError> {
        let old_files = self.get_commit_blobs(&context.from_hash).await?;
        let new_files = self.get_commit_blobs(&context.commit_hash).await?;
        let diff_files = self.cl_files_list(old_files, new_files).await?;

        let changes = self.build_changes(&context.repo_path, diff_files)?;

        let builds = vec![BuildInfo {
            changes: changes.clone(),
        }];

        Ok(builds)
    }

    fn build_changes(
        &self,
        cl_path: &str,
        cl_diff_files: Vec<crate::model::change_list::ClDiffFile>,
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
                let path = ProjectRelativePath::from_abs(&s.path, path_str).ok_or_else(|| {
                    MegaError::Other(format!("Invalid project-relative path: {}", s.path))
                })?;
                let status = if s.action == "new" {
                    Status::Added(path)
                } else if s.action == "deleted" {
                    Status::Removed(path)
                } else if s.action == "modified" {
                    Status::Modified(path)
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
    ) -> Result<Vec<crate::model::change_list::ClDiffFile>, MegaError> {
        let api_service = MonoApiService {
            storage: self.storage.clone(),
            git_object_cache: self.git_object_cache.clone(),
        };
        api_service.cl_files_list(old_files, new_files).await
    }
}

#[async_trait]
impl TriggerHandler for GitPushHandler {
    async fn handle(&self, context: &TriggerContext) -> Result<BuildTrigger, MegaError> {
        let builds = self.get_builds_for_commit(context).await?;

        let cl_link = context.cl_link.clone().unwrap_or_else(|| {
            format!(
                "push-{}-{}",
                Utc::now().timestamp_millis(),
                &context.commit_hash[..8.min(context.commit_hash.len())]
            )
        });

        Ok(BuildTrigger {
            trigger_type: context.trigger_type,
            trigger_source: context.trigger_source,
            trigger_time: Utc::now(),
            payload: BuildTriggerPayload::GitPush(crate::build_trigger::GitPushPayload {
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
        BuildTriggerType::GitPush
    }
}
