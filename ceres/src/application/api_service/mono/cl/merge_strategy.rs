//! CL merge strategy and monorepo path bootstrap helpers.

use std::path::{Component, Path};

use callisto::mega_cl;
use common::{errors::MegaError, utils::ZERO_ID};
use git_internal::{errors::GitError, internal::object::commit::Commit};
use jupiter::utils::converter::FromMegaModel;

use crate::api_service::{mono::MonoApiService, tree_ops};

/// How a CL should be applied onto monorepo main.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClMergeStrategy {
    /// Apply file-level diff onto the path main baseline (web edits, incremental pushes).
    FileDiff,
    /// Replace the CL path subtree with `cl.to_hash` root tree (GitHub import / new directory).
    SubtreeReplace,
}

impl ClMergeStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FileDiff => "file_diff",
            Self::SubtreeReplace => "subtree_replace",
        }
    }
}

/// Returns true when the path has no `refs/heads/main` row yet.
pub async fn path_lacks_main_ref(service: &MonoApiService, path: &str) -> Result<bool, MegaError> {
    Ok(service
        .storage
        .mono_storage()
        .get_main_ref(path)
        .await?
        .is_none())
}

/// Enumerate strict prefixes for a normalized repo path, e.g. `/project/mega` → [`/project`].
pub fn path_prefixes(path: &str) -> Vec<String> {
    let normalized = path.trim_end_matches('/');
    if normalized.is_empty() || normalized == "/" {
        return Vec::new();
    }
    let components: Vec<&str> = Path::new(normalized)
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();
    let mut out = Vec::new();
    let mut buf = String::from("/");
    for (idx, comp) in components.iter().enumerate() {
        if idx == components.len() - 1 {
            break;
        }
        if buf == "/" {
            buf.push_str(comp);
        } else {
            buf.push('/');
            buf.push_str(comp);
        }
        out.push(buf.clone());
    }
    out
}

/// Sync path-level main refs for each strict prefix after the root tree changed.
pub async fn sync_path_prefix_main_refs(
    service: &MonoApiService,
    path: &str,
) -> Result<(), MegaError> {
    for prefix in path_prefixes(path) {
        let hash = crate::code_edit::utils::create_repo_commit(&service.storage, &prefix).await?;
        if hash == ZERO_ID {
            return Err(MegaError::Other(format!(
                "Failed to sync main ref for prefix {prefix}"
            )));
        }
    }
    Ok(())
}

/// Bootstrap a new monorepo path: attach under `/`, sync prefix refs, create path main baseline.
pub async fn bootstrap_monorepo_path(
    service: &MonoApiService,
    path: &str,
    cl: Option<&mega_cl::Model>,
) -> Result<String, MegaError> {
    let mono_storage = service.storage.mono_storage();
    if let Some(existing) = mono_storage.get_main_ref(path).await? {
        if let Some(cl) = cl
            && cl.from_hash == ZERO_ID
        {
            service
                .storage
                .cl_storage()
                .update_cl_hash(cl.clone(), &existing.ref_commit_hash, &cl.to_hash)
                .await?;
        }
        return Ok(existing.ref_commit_hash);
    }

    service.attach_project_path_to_monorepo_root(path).await?;
    sync_path_prefix_main_refs(service, path).await?;

    let baseline_hash = crate::code_edit::utils::create_repo_commit(&service.storage, path).await?;
    if baseline_hash == ZERO_ID {
        return Err(MegaError::Other(format!(
            "Failed to create main ref baseline for {path}"
        )));
    }

    if let Some(cl) = cl
        && cl.from_hash == ZERO_ID
    {
        service
            .storage
            .cl_storage()
            .update_cl_hash(cl.clone(), &baseline_hash, &cl.to_hash)
            .await?;
    }

    Ok(baseline_hash)
}

/// Prepare a CL path for merge (bootstrap new `/project/*` directories).
pub async fn prepare_cl_path_for_merge(
    service: &MonoApiService,
    cl: &mut mega_cl::Model,
) -> Result<(), MegaError> {
    if !cl.path.starts_with("/project/") {
        return Ok(());
    }

    bootstrap_monorepo_path(service, &cl.path, Some(cl)).await?;

    if let Some(fresh) = service.storage.cl_storage().get_cl(&cl.link).await? {
        *cl = fresh;
    }
    Ok(())
}

pub async fn resolve_merge_strategy(
    service: &MonoApiService,
    cl: &mega_cl::Model,
) -> Result<ClMergeStrategy, MegaError> {
    if cl.from_hash == ZERO_ID {
        return Ok(ClMergeStrategy::SubtreeReplace);
    }

    if path_lacks_main_ref(service, &cl.path).await? {
        return Ok(ClMergeStrategy::SubtreeReplace);
    }

    if is_gitkeep_baseline(service, &cl.from_hash).await? {
        return Ok(ClMergeStrategy::SubtreeReplace);
    }

    Ok(ClMergeStrategy::FileDiff)
}

async fn is_gitkeep_baseline(
    service: &MonoApiService,
    commit_hash: &str,
) -> Result<bool, MegaError> {
    let blobs = service.get_commit_blobs(commit_hash).await?;
    if blobs.is_empty() {
        return Ok(true);
    }
    Ok(blobs.len() == 1
        && blobs[0]
            .0
            .file_name()
            .is_some_and(|name| name == ".gitkeep"))
}

/// Returns true when the final path segment is not yet present in the monorepo tree.
pub async fn needs_path_tree_attach(
    service: &MonoApiService,
    path: &Path,
) -> Result<bool, MegaError> {
    let parent = match path.parent() {
        Some(p) if p.as_os_str().is_empty() || p == Path::new("/") => Path::new("/"),
        Some(p) => p,
        None => return Ok(false),
    };
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Ok(false);
    };
    let parent_tree = match tree_ops::search_tree_by_path(service, parent, None).await? {
        Some(tree) => tree,
        None => return Ok(true),
    };
    Ok(!parent_tree
        .tree_items
        .iter()
        .any(|item| item.name == name && item.is_tree()))
}

/// Leaf tree hash to mount at `cl.path` for merge.
pub async fn resolve_merge_leaf_tree_id(
    service: &MonoApiService,
    cl: &mega_cl::Model,
    strategy: ClMergeStrategy,
) -> Result<git_internal::hash::ObjectHash, GitError> {
    let storage = service.storage.mono_storage();

    match strategy {
        ClMergeStrategy::SubtreeReplace => {
            let commit_model = storage
                .get_commit_by_hash(&cl.to_hash)
                .await
                .map_err(|e| GitError::CustomError(format!("Failed to get commit: {e}")))?
                .ok_or_else(|| {
                    GitError::CustomError(format!("Commit not found: {}", cl.to_hash))
                })?;
            let commit = Commit::from_mega_model(commit_model);
            Ok(commit.tree_id)
        }
        ClMergeStrategy::FileDiff => {
            let main_ref = storage
                .get_main_ref(&cl.path)
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?
                .ok_or_else(|| GitError::CustomError("Main ref not found".to_string()))?;

            let old_blobs = service
                .get_commit_blobs(&cl.from_hash)
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?;
            let new_blobs = service
                .get_commit_blobs(&cl.to_hash)
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?;
            let cl_changed = service
                .cl_files_list(old_blobs, new_blobs)
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?;

            let merged_commit_hash = service
                .apply_changes_as_single_commit(cl, &cl_changed, &main_ref.ref_commit_hash)
                .await?;

            let merged = storage
                .get_commit_by_hash(&merged_commit_hash)
                .await
                .map_err(|e| GitError::CustomError(e.to_string()))?
                .ok_or_else(|| {
                    GitError::CustomError(format!("Merged commit not found: {merged_commit_hash}"))
                })?;
            Ok(Commit::from_mega_model(merged).tree_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{path_prefixes, ClMergeStrategy};

    #[test]
    fn path_prefixes_returns_strict_prefixes() {
        assert_eq!(path_prefixes("/project/mega"), vec!["/project".to_string()]);
        assert_eq!(path_prefixes("/project"), Vec::<String>::new());
        assert_eq!(path_prefixes("/"), Vec::<String>::new());
    }

    #[test]
    fn cl_merge_strategy_as_str() {
        assert_eq!(ClMergeStrategy::FileDiff.as_str(), "file_diff");
        assert_eq!(ClMergeStrategy::SubtreeReplace.as_str(), "subtree_replace");
    }
}
