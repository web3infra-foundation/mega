use std::{path::PathBuf, sync::Arc};

use git_internal::{
    errors::GitError,
    internal::object::tree::{TreeItem, TreeItemMode},
};
use tokio::sync::Mutex;

use crate::api_service::{ApiHandler, cache::GitObjectCache, history, tree_ops};
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
    let start_commit = crate::api_service::resolve_start_commit(handler, refs).await?;

    let cache = Arc::new(Mutex::new(GitObjectCache::default()));

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
            cache,
        )
        .await?;

        let mut commit_info: LatestCommitInfo = commit.clone().into();

        // Apply username binding if available
        apply_username_binding(handler, &commit.id.to_string(), &mut commit_info).await;

        return Ok(commit_info);
    }

    // 2) If not a directory, try as file path
    // Use unified last-modification logic
    match history::resolve_last_modification_by_path(handler, &path, start_commit, cache).await {
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
