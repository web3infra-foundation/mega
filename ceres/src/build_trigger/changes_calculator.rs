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

fn detect_single_level_prefixed_candidate(repo_root: &Path, normalized: &str) -> Option<String> {
    if normalized.is_empty() || normalized.contains('/') {
        return None;
    }
    if !repo_root.exists() || repo_root.join(normalized).exists() {
        return None;
    }

    let mut candidate: Option<String> = None;
    let entries = std::fs::read_dir(repo_root).ok()?;
    for entry in entries.flatten() {
        if !entry.file_type().ok()?.is_dir() {
            continue;
        }
        let dir_name = entry.file_name();
        let dir_name = dir_name.to_str()?;
        let prefixed = format!("{dir_name}/{normalized}");
        if !repo_root.join(&prefixed).exists() {
            continue;
        }

        if candidate.is_some() {
            return None;
        }
        candidate = Some(prefixed);
    }

    candidate
}

fn monitor_possible_repo_prefix_mismatch(repo_path: &str, raw: &Path, normalized: &str) {
    let repo_root = Path::new(repo_path);
    if !repo_root.exists() || normalized.is_empty() || repo_root.join(normalized).exists() {
        return;
    }

    let Some(prefixed_candidate) = detect_single_level_prefixed_candidate(repo_root, normalized)
    else {
        return;
    };

    tracing::warn!(
        monitor_event = "build_change_path_prefix_mismatch",
        repo_path = %repo_path,
        raw_path = %raw.display(),
        normalized_path = %normalized,
        suggested_path = %prefixed_candidate,
        "Detected possible change-path prefix drift after normalization."
    );
}

#[cfg(test)]
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
        let normalized = normalize_change_path_for_repo_with_prefix(
            repo_prefix,
            repo_prefix_with_slash.as_deref(),
            path,
        );
        if let Some(normalized_path) = &normalized {
            monitor_possible_repo_prefix_mismatch(repo_path, path, normalized_path.as_str());
        }
        normalized
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
    use std::{fs, str::FromStr};

    use git_internal::hash::ObjectHash;
    use tempfile::TempDir;

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

    #[test]
    fn test_detect_single_level_prefixed_candidate_returns_unique_match() {
        let tempdir = TempDir::new().expect("create tempdir");
        let repo_root = tempdir.path();
        fs::create_dir_all(repo_root.join("src")).expect("create src dir");
        fs::write(repo_root.join("src/main.rs"), "fn main() {}\n").expect("write source file");

        assert_eq!(
            detect_single_level_prefixed_candidate(repo_root, "main.rs"),
            Some("src/main.rs".to_string())
        );
    }

    #[test]
    fn test_detect_single_level_prefixed_candidate_rejects_ambiguous_matches() {
        let tempdir = TempDir::new().expect("create tempdir");
        let repo_root = tempdir.path();
        fs::create_dir_all(repo_root.join("src")).expect("create src dir");
        fs::create_dir_all(repo_root.join("examples")).expect("create examples dir");
        fs::write(repo_root.join("src/main.rs"), "fn src() {}\n").expect("write src file");
        fs::write(repo_root.join("examples/main.rs"), "fn ex() {}\n").expect("write examples file");

        assert_eq!(
            detect_single_level_prefixed_candidate(repo_root, "main.rs"),
            None
        );
    }

    #[test]
    fn test_detect_single_level_prefixed_candidate_returns_none_when_direct_path_exists() {
        let tempdir = TempDir::new().expect("create tempdir");
        let repo_root = tempdir.path();
        fs::create_dir_all(repo_root.join("src")).expect("create src dir");
        fs::write(repo_root.join("main.rs"), "fn root() {}\n").expect("write root file");
        fs::write(repo_root.join("src/main.rs"), "fn nested() {}\n").expect("write nested file");

        assert_eq!(
            detect_single_level_prefixed_candidate(repo_root, "main.rs"),
            None
        );
    }
}
