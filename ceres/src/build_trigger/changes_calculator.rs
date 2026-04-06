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
    model::change_list::ClDiffFile,
};

fn to_monorepo_relative_path(repo_path: &str, path: &Path) -> ProjectRelativePath {
    let rel = path
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string();
    let repo_prefix = repo_path.trim_matches('/');
    let monorepo_relative = if repo_prefix.is_empty()
        || rel == repo_prefix
        || rel.starts_with(&format!("{repo_prefix}/"))
    {
        rel
    } else if rel.is_empty() {
        repo_prefix.to_string()
    } else {
        format!("{repo_prefix}/{rel}")
    };
    ProjectRelativePath::new(&monorepo_relative)
}

fn build_changes_for_repo(
    repo_path: &str,
    cl_diff_files: Vec<ClDiffFile>,
) -> Result<Vec<Status<ProjectRelativePath>>, MegaError> {
    // Orion workers resolve changed files against Buck inputs using
    // monorepo-root-relative paths.
    let to_project_relative = |path: &Path| -> Result<ProjectRelativePath, MegaError> {
        Ok(to_monorepo_relative_path(repo_path, path))
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

        let changes = build_changes_for_repo(&context.repo_path, diff_files)?;

        Ok(changes)
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use git_internal::hash::ObjectHash;

    use super::*;

    #[test]
    fn test_to_monorepo_relative_path_prefixes_subrepo_changes() {
        assert_eq!(
            to_monorepo_relative_path("/project/buck2_test", &PathBuf::from("src/main.rs")),
            ProjectRelativePath::new("project/buck2_test/src/main.rs")
        );
        assert_eq!(
            to_monorepo_relative_path(
                "/project/buck2_test",
                &PathBuf::from("project/buck2_test/src/generated.rs")
            ),
            ProjectRelativePath::new("project/buck2_test/src/generated.rs")
        );
    }

    #[test]
    fn test_to_monorepo_relative_path_keeps_root_repo_paths_unchanged() {
        assert_eq!(
            to_monorepo_relative_path("/", &PathBuf::from("common/lib.rs")),
            ProjectRelativePath::new("common/lib.rs")
        );
    }

    #[test]
    fn test_build_changes_applies_monorepo_prefix_to_all_statuses() {
        let changes = build_changes_for_repo(
            "/project/buck2_test",
            vec![
                ClDiffFile::Modified(
                    PathBuf::from("src/main.rs"),
                    ObjectHash::from_str("1111111111111111111111111111111111111111").unwrap(),
                    ObjectHash::from_str("2222222222222222222222222222222222222222").unwrap(),
                ),
                ClDiffFile::New(
                    PathBuf::from("src/generated.rs"),
                    ObjectHash::from_str("3333333333333333333333333333333333333333").unwrap(),
                ),
                ClDiffFile::Deleted(
                    PathBuf::from("README.md"),
                    ObjectHash::from_str("4444444444444444444444444444444444444444").unwrap(),
                ),
            ],
        )
        .unwrap();

        assert_eq!(
            changes,
            vec![
                Status::Modified(ProjectRelativePath::new("project/buck2_test/src/main.rs")),
                Status::Added(ProjectRelativePath::new(
                    "project/buck2_test/src/generated.rs"
                )),
                Status::Removed(ProjectRelativePath::new("project/buck2_test/README.md")),
            ]
        );
    }
}
