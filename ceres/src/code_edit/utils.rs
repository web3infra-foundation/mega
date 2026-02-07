use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    vec,
};

use callisto::mega_cl;
use common::{errors::MegaError, utils::ZERO_ID};
use git_internal::{hash::ObjectHash, internal::object::commit::Commit};
use jupiter::{storage::Storage, utils::converter::FromMegaModel};

use crate::{
    api_service::{ApiHandler, commit_ops},
    model::change_list::ClDiffFile,
};

pub async fn cl_files_list(
    old_files: Vec<(PathBuf, ObjectHash)>,
    new_files: Vec<(PathBuf, ObjectHash)>,
) -> Result<Vec<ClDiffFile>, MegaError> {
    let old_files: HashMap<PathBuf, ObjectHash> = old_files.into_iter().collect();
    let new_files: HashMap<PathBuf, ObjectHash> = new_files.into_iter().collect();
    let unions: HashSet<PathBuf> = old_files.keys().chain(new_files.keys()).cloned().collect();
    let mut res = vec![];
    for path in unions {
        let old_hash = old_files.get(&path);
        let new_hash = new_files.get(&path);
        match (old_hash, new_hash) {
            (None, None) => {}
            (None, Some(new)) => res.push(ClDiffFile::New(path, *new)),
            (Some(old), None) => res.push(ClDiffFile::Deleted(path, *old)),
            (Some(old), Some(new)) => {
                if old == new {
                    continue;
                } else {
                    res.push(ClDiffFile::Modified(path, *old, *new));
                }
            }
        }
    }

    // Sort the results
    res.sort_by(|a, b| {
        a.path()
            .cmp(b.path())
            .then_with(|| a.kind_weight().cmp(&b.kind_weight()))
    });
    Ok(res)
}

pub async fn get_repo_latest_commit(
    storage: &Storage,
    repo_path: &str,
) -> Result<Commit, MegaError> {
    let storage = storage.mono_storage();
    let refs = storage.get_main_ref(repo_path).await?.unwrap();
    Ok(Commit::from_mega_model(
        storage
            .get_commit_by_hash(&refs.ref_commit_hash)
            .await?
            .unwrap_or_else(|| panic!("can't fetch commit by hash {}", &refs.ref_commit_hash)),
    ))
}

/// Get list of files changed between from_hash and to_hash commits.
/// Returns paths relative to the CL root directory with forward slashes.
pub async fn get_changed_files<T: ApiHandler>(
    handler: &T,
    cl: &mega_cl::Model,
) -> Result<Vec<String>, MegaError> {
    let from_commit = handler.get_commit_by_hash(&cl.from_hash).await?;
    let to_commit = handler.get_commit_by_hash(&cl.to_hash).await?;
    let old_files = commit_ops::collect_commit_blobs(handler, &from_commit).await?;
    let new_files = commit_ops::collect_commit_blobs(handler, &to_commit).await?;
    let changed = cl_files_list(old_files, new_files).await?;

    // Normalize CL root path to use forward slashes
    let cl_root = cl.path.to_string().replace('\\', "/");
    let cl_root_normalized = cl_root.trim_start_matches('/');

    let file_paths: Vec<String> = changed
        .iter()
        .map(|f| {
            let full_path = f.path().to_string_lossy().replace('\\', "/");
            let full_path_normalized = full_path.trim_start_matches('/');

            // Strip CL root prefix to get relative path
            if let Some(rel) = full_path_normalized.strip_prefix(cl_root_normalized) {
                rel.trim_start_matches('/').to_string()
            } else {
                full_path.to_string()
            }
        })
        .collect();

    Ok(file_paths)
}

/// Collect Cedar policy files from directories of all changed files.
/// Also collects policies from parent directories up to Monorepo root for inheritance.
/// Tries from_hash first for security, then falls back to to_hash for new directories.
/// Returns list of (policy_path, content) tuples, ordered from root to leaf.
pub async fn collect_policy_contents<T: ApiHandler>(
    handler: &T,
    cl: &mega_cl::Model,
    changed_files: &[String],
) -> Vec<(PathBuf, String)> {
    let mut all_policy_dirs: HashSet<PathBuf> = HashSet::new();

    // Always include the CL root directory
    all_policy_dirs.insert(PathBuf::new());

    // Collect ancestor directories from all changed files
    for file_path in changed_files {
        let relative_path = file_path.trim_start_matches('/').replace('\\', "/");
        let path = PathBuf::from(&relative_path);

        let parent = path.parent().unwrap_or(std::path::Path::new(""));

        // Skip .cedar directory itself, use its parent
        let logical_parent = if parent.file_name().map(|n| n == ".cedar").unwrap_or(false) {
            parent.parent().unwrap_or(std::path::Path::new(""))
        } else {
            parent
        };

        for ancestor in logical_parent.ancestors() {
            let ancestor_str = ancestor.to_string_lossy();
            if ancestor_str.contains(".cedar") {
                continue;
            }
            let normalized = PathBuf::from(ancestor_str.replace('\\', "/"));
            all_policy_dirs.insert(normalized);
        }
    }

    // Sort by depth for correct override semantics (root policies first)
    let mut sorted_dirs: Vec<PathBuf> = all_policy_dirs.into_iter().collect();
    sorted_dirs.sort_by_key(|p| p.components().count());

    let mut policy_contents: Vec<(PathBuf, String)> = Vec::new();
    let mut seen_policies: HashSet<String> = HashSet::new();

    let self_path_str = cl.path.to_string().replace('\\', "/");
    let self_path_normalized = self_path_str.trim_start_matches('/');

    // Step 1: Collect parent policies from Monorepo root down to CL directory
    // This enables inheritance from e.g. /project/.cedar/policies.cedar
    let parent_dirs = collect_parent_policy_dirs(cl);

    for parent_dir in parent_dirs {
        // Use absolute path as key to avoid collision with CL-local policies
        let absolute_policy_path = if parent_dir.is_empty() {
            "/.cedar/policies.cedar".to_string()
        } else {
            format!("/{}/.cedar/policies.cedar", parent_dir)
        };

        if seen_policies.contains(&absolute_policy_path) {
            continue;
        }

        // For parent policies, we use a rooted MonoApiService
        if let Some(content) = get_parent_policy_content(handler, &parent_dir).await {
            seen_policies.insert(absolute_policy_path.clone());
            policy_contents.push((PathBuf::from(&absolute_policy_path), content));
        }
    }

    // Step 2: Collect policies within the CL directory
    for dir in sorted_dirs {
        let policy_relative_path = if dir.as_os_str().is_empty() {
            ".cedar/policies.cedar".to_string()
        } else {
            let dir_str = dir.to_string_lossy().replace('\\', "/");
            format!("{}/.cedar/policies.cedar", dir_str)
        };

        // Build absolute path for deduplication
        let absolute_policy_path = if self_path_normalized.is_empty() {
            format!("/{}", policy_relative_path)
        } else {
            format!("/{}/{}", self_path_normalized, policy_relative_path)
        };

        // Skip if already seen from parent collection
        if seen_policies.contains(&absolute_policy_path) {
            continue;
        }

        let lookup_path = PathBuf::from(&policy_relative_path);

        // Fetch policy content: try from_hash for existing, fall back to to_hash for new
        let content = if cl.from_hash != ZERO_ID {
            if let Ok(Some(content)) = handler
                .get_blob_as_string(lookup_path.clone(), Some(&cl.from_hash))
                .await
            {
                Some(content)
            } else {
                handler
                    .get_blob_as_string(lookup_path, Some(&cl.to_hash))
                    .await
                    .ok()
                    .flatten()
            }
        } else {
            handler
                .get_blob_as_string(lookup_path, Some(&cl.to_hash))
                .await
                .ok()
                .flatten()
        };

        if let Some(content) = content {
            seen_policies.insert(absolute_policy_path.clone());
            policy_contents.push((PathBuf::from(&absolute_policy_path), content));
        }
    }

    policy_contents
}

/// Collect parent directory paths from Monorepo root to CL directory (exclusive).
pub fn collect_parent_policy_dirs(cl: &mega_cl::Model) -> Vec<String> {
    let self_path_str = cl.path.to_string().replace('\\', "/");
    let self_path_normalized = self_path_str.trim_start_matches('/');

    if self_path_normalized.is_empty() {
        return vec![];
    }

    let mut parent_dirs = Vec::new();
    let components: Vec<&str> = self_path_normalized.split('/').collect();

    // Add root directory
    parent_dirs.push(String::new());

    // Add each parent level except the CL directory itself
    let mut current_path = String::new();
    for (i, component) in components.iter().enumerate() {
        if i == components.len() - 1 {
            break;
        }
        if current_path.is_empty() {
            current_path = component.to_string();
        } else {
            current_path = format!("{}/{}", current_path, component);
        }
        parent_dirs.push(current_path.clone());
    }

    parent_dirs
}

/// Get policy content from a parent directory using storage directly.
pub async fn get_parent_policy_content<T: ApiHandler>(
    handler: &T,
    parent_dir: &str,
) -> Option<String> {
    let storage = handler.get_context();
    let mono_storage = storage.mono_storage();

    // Get the main ref for the parent directory
    let parent_path = if parent_dir.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", parent_dir)
    };

    let refs = mono_storage.get_main_ref(&parent_path).await.ok()??;

    // Look up .cedar/policies.cedar in the parent directory
    let policy_path = PathBuf::from(".cedar/policies.cedar");
    handler
        .get_blob_as_string(policy_path, Some(&refs.ref_commit_hash))
        .await
        .ok()
        .flatten()
}
