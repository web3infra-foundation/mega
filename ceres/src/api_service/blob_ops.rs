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
        };
    }
    Ok(None)
}
