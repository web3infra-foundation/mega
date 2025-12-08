use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use common::model::DiffItem;
use git_internal::{
    diff::Diff as GitDiff,
    errors::GitError,
    hash::SHA1,
    internal::object::{blob::Blob, tree::TreeItemMode},
};

use crate::api_service::ApiHandler;
use crate::api_service::tree_ops;
use crate::model::git::DiffPreviewPayload;
use futures::{StreamExt, stream};

/// Convenience: get file blob oid at HEAD (or provided refs) by path
pub async fn get_file_blob_id<T: ApiHandler + ?Sized>(
    handler: &T,
    path: &Path,
    refs: Option<&str>,
) -> Result<Option<SHA1>, GitError> {
    let parent = path.parent().unwrap_or(Path::new("/"));
    if let Some(tree) = tree_ops::search_tree_by_path(handler, parent, refs).await? {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if let Some(item) = tree.tree_items.into_iter().find(|x| x.name == name)
            && item.mode == TreeItemMode::Blob
        {
            return Ok(Some(item.id));
        }
    }
    Ok(None)
}

/// Get blob IDs for multiple file paths in batch
///
/// # Arguments
/// * `handler` - API handler implementing ApiHandler trait
/// * `paths` - Slice of file paths to query
/// * `refs` - Optional commit hash or ref name
///
/// # Returns
/// HashMap mapping file paths to their blob IDs (as SHA1)
/// Files not found will not be in the result (use contains_key to check)
pub async fn get_files_blob_ids<T: ApiHandler + ?Sized>(
    handler: &T,
    paths: &[PathBuf],
    refs: Option<&str>,
) -> Result<HashMap<PathBuf, SHA1>, GitError> {
    if paths.is_empty() {
        return Ok(HashMap::new());
    }

    // Group paths by parent directory to minimize tree queries
    batch_query_via_trees(handler, paths, refs).await
}

/// Batch query blob IDs via Git tree structure
async fn batch_query_via_trees<T: ApiHandler + ?Sized>(
    handler: &T,
    paths: &[PathBuf],
    refs: Option<&str>,
) -> Result<HashMap<PathBuf, SHA1>, GitError> {
    // Group paths by parent directory
    let mut paths_by_parent: HashMap<PathBuf, Vec<(PathBuf, String)>> = HashMap::new();
    for path in paths {
        let parent = path.parent().unwrap_or(Path::new("/"));
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        paths_by_parent
            .entry(parent.to_path_buf())
            .or_default()
            .push((path.clone(), file_name));
    }

    // Calculate max concurrent queries based on database connection pool size
    let max_concurrent_tree_queries = {
        let storage = handler.get_context();
        let max_connection = storage.config().database.max_connection as usize;

        // Use 50% of max_connection, with bounds: min 4, max = max_connection
        let calculated = (max_connection * 50) / 100;
        calculated.max(4).min(max_connection)
    };

    let tree_queries: Vec<_> = paths_by_parent
        .keys()
        .map(|parent_path| {
            let parent_path = parent_path.clone();
            async move {
                let tree_result = tree_ops::search_tree_by_path(handler, &parent_path, refs).await;
                (parent_path, tree_result)
            }
        })
        .collect();

    // Execute tree queries in parallel (limit concurrency to avoid overwhelming DB)
    let tree_results: Vec<_> = stream::iter(tree_queries)
        .buffer_unordered(max_concurrent_tree_queries)
        .collect()
        .await;

    // Extract blob IDs from trees
    let mut result = HashMap::new();
    for (parent_path, tree_result) in tree_results {
        match tree_result {
            Ok(Some(tree)) => {
                if let Some(paths_in_parent) = paths_by_parent.get(&parent_path) {
                    for (file_path, file_name) in paths_in_parent {
                        if let Some(item) = tree.tree_items.iter().find(|x| x.name == *file_name)
                            && item.mode == TreeItemMode::Blob
                        {
                            result.insert(file_path.clone(), item.id);
                        }
                    }
                }
            }
            Ok(None) => {
                // Parent tree not found, skip files in this directory
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to get tree for parent path {}: {}",
                    parent_path.display(),
                    e
                );
            }
        }
    }

    Ok(result)
}

/// Preview unified diff for a single file change
pub async fn preview_file_diff<T: ApiHandler + ?Sized>(
    handler: &T,
    payload: DiffPreviewPayload,
) -> Result<Option<DiffItem>, GitError> {
    let path = PathBuf::from(&payload.path);
    // old oid and content
    let old_oid_opt = get_file_blob_id(handler, &path, Some(payload.refs.as_str())).await?;
    let old_entry = if let Some(oid) = old_oid_opt {
        vec![(path.clone(), oid)]
    } else {
        Vec::new()
    };
    let new_blob = Blob::from_content(&payload.content);
    let new_entry = vec![(path.clone(), new_blob.id)];

    // local content reader: use DB for old oid and memory for new
    let mut cache: HashMap<SHA1, Vec<u8>> = HashMap::new();
    if let Some(oid) = old_oid_opt
        && let Some(model) = handler.get_raw_blob_by_hash(&oid.to_string()).await?
    {
        cache.insert(oid, model.data.unwrap_or_default());
    }
    cache.insert(new_blob.id, payload.content.into_bytes());

    let read = |_: &PathBuf, oid: &SHA1| -> Vec<u8> { cache.get(oid).cloned().unwrap_or_default() };
    let mut items: Vec<DiffItem> = GitDiff::diff(old_entry, new_entry, Vec::new(), read)
        .into_iter()
        .map(DiffItem::from)
        .collect();
    Ok(items.pop())
}

pub async fn get_blob_as_string<T: ApiHandler + ?Sized>(
    handler: &T,
    file_path: PathBuf,
    refs: Option<&str>,
) -> Result<Option<String>, GitError> {
    let filename = file_path.file_name().unwrap().to_str().unwrap();
    let parent = file_path.parent().unwrap();
    if let Some(tree) = tree_ops::search_tree_by_path(handler, parent, refs).await?
        && let Some(item) = tree.tree_items.into_iter().find(|x| x.name == filename)
    {
        match handler.get_raw_blob_by_hash(&item.id.to_string()).await {
            Ok(Some(model)) => {
                return Ok(Some(String::from_utf8(model.data.unwrap()).unwrap()));
            }
            _ => return Ok(None),
        }
    }
    Ok(None)
}
