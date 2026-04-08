use std::{
    path::{Component, Path, PathBuf},
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

fn is_safe_normalized_path(path: &str) -> bool {
    path.is_empty()
        || (!path.contains("//")
            && Path::new(path)
                .components()
                .all(|component| matches!(component, Component::Normal(_))))
}

fn normalize_change_path_for_repo_with_prefix(
    repo_prefix: &str,
    repo_prefix_with_slash: Option<&str>,
    path: &Path,
) -> Option<ProjectRelativePath> {
    let raw = path
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string();

    let normalized = if repo_prefix.is_empty() {
        raw
    } else if raw == repo_prefix {
        String::new()
    } else if let Some(prefix) = repo_prefix_with_slash {
        if let Some(stripped) = raw.strip_prefix(prefix) {
            stripped.to_string()
        } else {
            raw
        }
    } else {
        raw
    };

    if !is_safe_normalized_path(&normalized) {
        tracing::warn!(
            path = %normalized,
            "Dropping unsafe build change path after normalization."
        );
        return None;
    }

    Some(ProjectRelativePath::new(&normalized))
}

fn normalize_change_path_for_repo(repo_path: &str, path: &Path) -> Option<ProjectRelativePath> {
    let repo_prefix = repo_path.trim_matches('/');
    let repo_prefix_with_slash = (!repo_prefix.is_empty()).then(|| format!("{repo_prefix}/"));
    normalize_change_path_for_repo_with_prefix(repo_prefix, repo_prefix_with_slash.as_deref(), path)
}

fn push_change_if_valid(
    changes: &mut Vec<Status<ProjectRelativePath>>,
    status_builder: impl FnOnce(ProjectRelativePath) -> Status<ProjectRelativePath>,
    normalized: Option<ProjectRelativePath>,
) {
    if let Some(path) = normalized {
        changes.push(status_builder(path));
    }
}

fn build_changes_for_repo(
    repo_path: &str,
    cl_diff_files: Vec<ClDiffFile>,
) -> Result<Vec<Status<ProjectRelativePath>>, MegaError> {
    let repo_prefix = repo_path.trim_matches('/');
    let repo_prefix_with_slash = (!repo_prefix.is_empty()).then(|| format!("{repo_prefix}/"));
    let to_project_relative = |path: &Path| {
        normalize_change_path_for_repo_with_prefix(
            repo_prefix,
            repo_prefix_with_slash.as_deref(),
            path,
        )
    };

    let mut counter_changes = Vec::new();
    for change in cl_diff_files {
        match change {
            ClDiffFile::New(path, _) => {
                push_change_if_valid(
                    &mut counter_changes,
                    Status::Added,
                    to_project_relative(&path),
                );
            }
            ClDiffFile::Deleted(path, _) => {
                push_change_if_valid(
                    &mut counter_changes,
                    Status::Removed,
                    to_project_relative(&path),
                );
            }
            ClDiffFile::Modified(path, _, _) => {
                push_change_if_valid(
                    &mut counter_changes,
                    Status::Modified,
                    to_project_relative(&path),
                );
            }
            ClDiffFile::Renamed(old_path, new_path, _, _, _)
            | ClDiffFile::Moved(old_path, new_path, _, _, _) => {
                push_change_if_valid(
                    &mut counter_changes,
                    Status::Removed,
                    to_project_relative(&old_path),
                );
                push_change_if_valid(
                    &mut counter_changes,
                    Status::Added,
                    to_project_relative(&new_path),
                );
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
    fn test_normalize_change_path_for_repo_strips_repo_prefix_for_local_files() {
        assert_eq!(
            normalize_change_path_for_repo("/project/buck2_test", &PathBuf::from("src/main.rs")),
            Some(ProjectRelativePath::new("src/main.rs"))
        );
        assert_eq!(
            normalize_change_path_for_repo(
                "/project/buck2_test",
                &PathBuf::from("project/buck2_test/src/generated.rs")
            ),
            Some(ProjectRelativePath::new("src/generated.rs"))
        );
    }

    #[test]
    fn test_normalize_change_path_for_repo_keeps_external_shared_paths() {
        assert_eq!(
            normalize_change_path_for_repo("/project/buck2_test", &PathBuf::from("common/lib.rs")),
            Some(ProjectRelativePath::new("common/lib.rs"))
        );
    }

    #[test]
    fn test_normalize_change_path_for_repo_rejects_unsafe_paths() {
        assert_eq!(
            normalize_change_path_for_repo("/project/buck2_test", &PathBuf::from("../secret.rs")),
            None
        );
        assert_eq!(
            normalize_change_path_for_repo(
                "/project/buck2_test",
                &PathBuf::from("project/buck2_test/../../secret.rs")
            ),
            None
        );
        assert_eq!(
            normalize_change_path_for_repo(
                "/project/buck2_test",
                &PathBuf::from("project//buck2_test/src/main.rs")
            ),
            None
        );
    }

    #[test]
    fn test_build_changes_normalizes_repo_local_paths_and_keeps_external_paths() {
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
                ClDiffFile::Modified(
                    PathBuf::from("common/lib.rs"),
                    ObjectHash::from_str("5555555555555555555555555555555555555555").unwrap(),
                    ObjectHash::from_str("6666666666666666666666666666666666666666").unwrap(),
                ),
            ],
        )
        .unwrap();

        assert_eq!(
            changes,
            vec![
                Status::Modified(ProjectRelativePath::new("src/main.rs")),
                Status::Added(ProjectRelativePath::new("src/generated.rs")),
                Status::Removed(ProjectRelativePath::new("README.md")),
                Status::Modified(ProjectRelativePath::new("common/lib.rs")),
            ]
        );
    }

    #[test]
    fn test_build_changes_filters_unsafe_paths() {
        let changes = build_changes_for_repo(
            "/project/buck2_test",
            vec![
                ClDiffFile::Modified(
                    PathBuf::from("src/main.rs"),
                    ObjectHash::from_str("1111111111111111111111111111111111111111").unwrap(),
                    ObjectHash::from_str("2222222222222222222222222222222222222222").unwrap(),
                ),
                ClDiffFile::New(
                    PathBuf::from("../outside.rs"),
                    ObjectHash::from_str("3333333333333333333333333333333333333333").unwrap(),
                ),
            ],
        )
        .unwrap();

        assert_eq!(
            changes,
            vec![Status::Modified(ProjectRelativePath::new("src/main.rs"))]
        );
    }
}
