use std::{path::PathBuf, sync::Arc};

use git_internal::{
    errors::GitError,
    hash::SHA1,
    internal::object::{
        commit::Commit,
        tree::{TreeItem, TreeItemMode},
    },
};
use tokio::sync::Mutex;

use crate::api_service::{ApiHandler, cache::GitObjectCache, history, tree_ops};
use crate::model::git::{CommitBindingInfo, LatestCommitInfo};

pub async fn get_latest_commit<T: ApiHandler + ?Sized>(
    handler: &T,
    path: PathBuf,
) -> Result<LatestCommitInfo, GitError> {
    // 1) Try as directory path first
    if let Some(tree) = tree_ops::search_tree_by_path(handler, &path, None).await? {
        let commit = get_tree_relate_commit(handler, tree.id, path).await?;
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
    // basic validation for file path
    path.file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| GitError::CustomError("Invalid file path".to_string()))?;
    let parent = path
        .parent()
        .ok_or_else(|| GitError::CustomError("Invalid file path".to_string()))?;

    // parent must be a directory tree that exists
    if tree_ops::search_tree_by_path(handler, parent, None)
        .await?
        .is_none()
    {
        return Err(GitError::CustomError(
            "can't find target parent tree under latest commit".to_string(),
        ));
    };
    match history::resolve_latest_commit_for_file_path(handler, &path).await? {
        Some(commit) => {
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
        None => Err(GitError::CustomError(
            "[code:404] File not found".to_string(),
        )),
    }
}

pub async fn get_tree_relate_commit<T: ApiHandler + ?Sized>(
    handler: &T,
    t_hash: SHA1,
    path: PathBuf,
) -> Result<Commit, GitError> {
    let file_name = match path.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => {
            return Err(GitError::CustomError("Invalid Path Input".to_string()));
        }
    };

    let search_item = TreeItem::new(TreeItemMode::Tree, t_hash, file_name);
    let cache = Arc::new(Mutex::new(GitObjectCache::default()));
    let root_commit = Arc::new(handler.get_root_commit().await);

    let parent = match path.parent() {
        Some(p) => p,
        None => return Err(GitError::CustomError("Invalid Path Input".to_string())),
    };
    history::traverse_commit_history(handler, parent, root_commit, &search_item, cache).await
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
