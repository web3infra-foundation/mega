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

    // 1) Try as directory path first
    if let Some(tree) = tree_ops::search_tree_by_path(handler, &path, refs).await? {
        // Special handling for root directory
        let (dir_name, parent): (String, &std::path::Path) = if path.as_os_str().is_empty()
            || path == std::path::Path::new(".")
            || path == std::path::Path::new("/")
        {
            // For root directory, treat it as the tree itself with empty parent
            // This commit represents the root tree's last modification
            (String::from(""), std::path::Path::new(""))
        } else {
            // For non-root directories, extract name and parent normally
            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| GitError::CustomError("Invalid directory path".to_string()))?
                .to_string();
            let parent = path
                .parent()
                .ok_or_else(|| GitError::CustomError("Directory has no parent".to_string()))?;
            (dir_name, parent)
        };

        let dir_item = TreeItem::new(TreeItemMode::Tree, tree.id, dir_name);

        let cache = Arc::new(Mutex::new(GitObjectCache::default()));

        let commit = history::traverse_commit_history_for_last_modification(
            handler,
            parent,
            start_commit.clone(),
            &dir_item,
            cache,
        )
        .await?;

        let mut commit_info: LatestCommitInfo = commit.clone().into();

        // If commit has a username binding, prefer showing that username
        if let Ok(Some(binding)) = handler
            .build_commit_binding_info(&commit.id.to_string())
            .await
            && !binding.is_anonymous
            && binding.matched_username.is_some()
        {
            let username = binding.matched_username.unwrap();
            commit_info.author = username.clone();
            commit_info.committer = username;
        }

        return Ok(commit_info);
    }

    // 2) If not a directory, try as file path
    // Use unified last-modification logic
    let cache = Arc::new(Mutex::new(GitObjectCache::default()));

    match history::resolve_last_modification_by_path(handler, &path, start_commit, cache).await {
        Ok(commit) => {
            let mut commit_info: LatestCommitInfo = commit.clone().into();
            // If commit has a username binding, prefer showing that username
            if let Ok(Some(binding)) = handler
                .build_commit_binding_info(&commit.id.to_string())
                .await
                && !binding.is_anonymous
                && binding.matched_username.is_some()
            {
                let username = binding.matched_username.unwrap();
                commit_info.author = username.clone();
                commit_info.committer = username;
            }
            Ok(commit_info)
        }
        Err(e) => {
            // Preserve the original error message for better debugging
            tracing::debug!("File not found or error during traversal: {:?}", e);
            // If it's already a CustomError with [code:404], preserve it
            if let GitError::CustomError(msg) = &e
                && msg.starts_with("[code:404]")
            {
                return Err(e);
            }
            // Otherwise wrap with 404 code
            Err(GitError::CustomError(
                "[code:404] File not found".to_string(),
            ))
        }
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
