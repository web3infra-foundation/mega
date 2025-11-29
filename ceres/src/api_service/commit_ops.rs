use std::path::PathBuf;
use std::sync::Arc;

use git_internal::{
    errors::GitError,
    internal::object::{
        commit::Commit,
        tree::{TreeItem, TreeItemMode},
    },
};

use crate::api_service::{ApiHandler, history, tree_ops};
use crate::model::git::{CommitBindingInfo, LatestCommitInfo};

/// Get the latest commit that modified a file or directory.
///
/// This unified function handles both tag-based and commit-based browsing through
/// the `refs` parameter, ensuring consistent behavior across all code paths.
///
/// # Arguments
/// - `handler`: API handler for accessing Git data
/// - `path`: File or directory path to check
/// - `refs`: Optional reference (tag name or commit SHA). If None, uses default HEAD/root.
///
/// # Returns
/// The commit information for the last modification of the specified path.
pub async fn get_latest_commit<T: ApiHandler + ?Sized>(
    handler: &T,
    path: PathBuf,
    refs: Option<&str>,
) -> Result<LatestCommitInfo, GitError> {
    // Resolve the starting commit from refs
    let start_commit = resolve_start_commit(handler, refs).await?;

    // 1) Try as directory path first
    if let Some(tree) = tree_ops::search_tree_by_path(handler, &path, refs).await? {
        let is_repo_root = tree.id == start_commit.tree_id;
        // Special handling for root directory
        if is_repo_root
            || path.as_os_str().is_empty()
            || path == std::path::Path::new(".")
            || path == std::path::Path::new("/")
        {
            // For root directory, the start_commit itself is the last modification
            let mut commit_info: LatestCommitInfo = (*start_commit).clone().into();

            // Apply username binding if available
            apply_username_binding(handler, &start_commit.id.to_string(), &mut commit_info).await;

            return Ok(commit_info);
        }

        // For non-root directories, extract name and parent normally
        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| GitError::CustomError("Invalid directory path".to_string()))?
            .to_string();
        let parent = path
            .parent()
            .ok_or_else(|| GitError::CustomError("Directory has no parent".to_string()))?;

        let dir_item = TreeItem::new(TreeItemMode::Tree, tree.id, dir_name);

        let commit = history::traverse_commit_history_for_last_modification(
            handler,
            parent,
            start_commit.clone(),
            &dir_item,
        )
        .await?;

        let mut commit_info: LatestCommitInfo = commit.clone().into();

        // Apply username binding if available
        apply_username_binding(handler, &commit.id.to_string(), &mut commit_info).await;

        return Ok(commit_info);
    }

    // 2) If not a directory, try as file path
    // Use unified last-modification logic
    match history::resolve_last_modification_by_path(handler, &path, start_commit).await {
        Ok(commit) => {
            let mut commit_info: LatestCommitInfo = commit.clone().into();

            // Apply username binding if available
            apply_username_binding(handler, &commit.id.to_string(), &mut commit_info).await;

            Ok(commit_info)
        }
        Err(e) => {
            // Preserve the original error message for better debugging
            tracing::debug!("File not found or error during traversal: {:?}", e);
            match e {
                GitError::CustomError(ref msg) if msg.starts_with("[code:404]") => Err(e),
                _ => Err(GitError::CustomError(
                    "[code:404] File not found".to_string(),
                )),
            }
        }
    }
}

/// Apply username binding to commit info if available.
/// This replaces the Git commit author/committer with the bound username if:
/// - A binding exists for this commit
/// - The binding is not anonymous
/// - A matched username is available
async fn apply_username_binding<T: ApiHandler + ?Sized>(
    handler: &T,
    commit_id: &str,
    commit_info: &mut LatestCommitInfo,
) {
    if let Ok(Some(binding)) = handler.build_commit_binding_info(commit_id).await
        && !binding.is_anonymous
        && let Some(username) = binding.matched_username
    {
        commit_info.author = username.clone();
        commit_info.committer = username;
    }
}

/// Build commit binding information for a given commit SHA
pub async fn build_commit_binding_info<T: ApiHandler + ?Sized>(
    handler: &T,
    commit_sha: &str,
) -> Result<Option<CommitBindingInfo>, GitError> {
    let storage = handler.get_context();
    let commit_binding_storage = storage.commit_binding_storage();

    if let Ok(Some(binding_model)) = commit_binding_storage.find_by_sha(commit_sha).await {
        Ok(Some(CommitBindingInfo {
            matched_username: binding_model.matched_username,
            is_anonymous: binding_model.is_anonymous,
        }))
    } else {
        Ok(None)
    }
}

/// Resolves a reference string to a starting commit for history traversal.
///
/// This function provides unified logic for parsing different ref formats across all APIs.
/// It supports branch names, tags (with or without `refs/tags/` prefix), and commit SHAs.
///
/// # Arguments
/// - `handler`: The API handler providing Git operations
/// - `refs`: Optional reference string, which can be:
///   - `None` or empty string: returns root commit (HEAD)
///   - Branch name (e.g., `main`, `master`, `refs/heads/main`)
///   - Tag name with `refs/tags/` prefix (e.g., `refs/tags/v1.0.0`)
///   - Tag name without prefix (e.g., `v1.0.0`)
///   - Commit SHA (7-40 character hexadecimal, supporting short SHAs)
///
/// # Returns
/// - `Ok(Arc<Commit>)`: The resolved commit wrapped in an Arc for efficient sharing
/// - `Err(GitError)`: If the reference cannot be resolved to a valid commit
pub async fn resolve_start_commit<T: ApiHandler + ?Sized>(
    handler: &T,
    refs: Option<&str>,
) -> Result<Arc<Commit>, GitError> {
    // Handle None or empty refs: return HEAD (root commit)
    let Some(ref_str) = refs else {
        return Ok(Arc::new(handler.get_root_commit().await?));
    };

    let ref_str = ref_str.trim();
    if ref_str.is_empty() {
        return Ok(Arc::new(handler.get_root_commit().await?));
    }

    // Resolve main/master branch to root commit
    let branch_name = ref_str.strip_prefix("refs/heads/").unwrap_or(ref_str);
    if branch_name == "main" || branch_name == "master" {
        return Ok(Arc::new(handler.get_root_commit().await?));
    }

    // Try to resolve as tag (with or without refs/tags/ prefix)
    let tag_name = ref_str.strip_prefix("refs/tags/").unwrap_or(ref_str);
    if let Ok(Some(tag)) = handler.get_tag(None, tag_name.to_string()).await {
        return Ok(Arc::new(
            handler
                .get_commit_by_hash(&tag.object_id.to_string())
                .await?,
        ));
    }

    // Try to resolve as commit SHA (support short SHA: 7-40 hex digits)
    if (7..=40).contains(&ref_str.len()) && ref_str.chars().all(|c| c.is_ascii_hexdigit()) {
        let commit = handler.get_commit_by_hash(ref_str).await?;

        // Defensive: ensure the resolved commit actually matches the requested SHA
        // Support short SHAs by requiring the full id to start with the provided prefix.
        if !commit.id.to_string().starts_with(ref_str) {
            return Err(GitError::CustomError(format!(
                "Commit SHA '{}' not found",
                ref_str
            )));
        }

        return Ok(Arc::new(commit));
    }

    // Failed to resolve: return descriptive error
    Err(GitError::CustomError(format!(
        "Invalid reference '{}': not a valid branch, tag, or commit SHA",
        ref_str
    )))
}
