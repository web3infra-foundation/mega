use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use futures::{StreamExt, stream};
use git_internal::{
    DiffItem,
    diff::Diff as GitDiff,
    errors::GitError,
    hash::ObjectHash,
    internal::object::{blob::Blob, tree::TreeItemMode},
};
use hex;
use sha1::{Digest, Sha1};

use crate::{
    api_service::{ApiHandler, tree_ops},
    model::git::DiffPreviewPayload,
};

/// Convenience: get file blob oid at HEAD (or provided refs) by path
pub async fn get_file_blob_id<T: ApiHandler + ?Sized>(
    handler: &T,
    path: &Path,
    refs: Option<&str>,
) -> Result<Option<ObjectHash>, GitError> {
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
/// HashMap mapping file paths to ObjectHash blob IDs
/// Files not found will not be in the result (use contains_key to check)
pub async fn get_files_blob_ids<T: ApiHandler + ?Sized>(
    handler: &T,
    paths: &[PathBuf],
    refs: Option<&str>,
) -> Result<HashMap<PathBuf, ObjectHash>, GitError> {
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
) -> Result<HashMap<PathBuf, ObjectHash>, GitError> {
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
    // Use Storage-provided recommended concurrency limit
    let max_concurrent_tree_queries = handler.get_context().get_recommended_batch_concurrency();

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
    let mut cache: HashMap<ObjectHash, Vec<u8>> = HashMap::new();
    if let Some(oid) = old_oid_opt {
        let data = handler.get_raw_blob_by_hash(&oid.to_string()).await?;
        cache.insert(oid, data);
    }
    cache.insert(new_blob.id, payload.content.into_bytes());

    let read =
        |_: &PathBuf, oid: &ObjectHash| -> Vec<u8> { cache.get(oid).cloned().unwrap_or_default() };
    let mut items: Vec<DiffItem> = GitDiff::diff(old_entry, new_entry, Vec::new(), read);
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
            Ok(data) => {
                return Ok(Some(String::from_utf8(data).unwrap()));
            }
            _ => return Ok(None),
        }
    }
    Ok(None)
}

/// Extract raw content slice from Git blob data with strict validation and fallback
///
/// Git blob format: "blob <size>\0<content>"
/// This function attempts to parse Git blob format, but falls back to returning
/// raw content if validation fails. This ensures data is never lost.
///
/// # Validation Rules
/// 1. Must start with "blob "
/// 2. Must contain a null byte separator
/// 3. Size field must be valid ASCII digits
/// 4. Size must match actual content length
///
/// # Returns
/// Content slice (Git blob header stripped if valid, or original data if fallback).
/// This function never fails - it always returns a valid slice, either by stripping
/// the Git blob header or by returning the original data unchanged.
fn extract_raw_content_from_blob(blob_data: &[u8]) -> &[u8] {
    // If it doesn't start with "blob ", return as-is
    if !blob_data.starts_with(b"blob ") {
        return blob_data;
    }

    // Try to parse as Git blob format: "blob <size>\0<content>"
    if let Some(null_pos) = blob_data.iter().position(|&b| b == 0) {
        let size_bytes = &blob_data[5..null_pos];

        // Check if size field contains only ASCII digits
        if !size_bytes.iter().all(|&b| b.is_ascii_digit()) {
            return blob_data;
        }

        // Parse size as usize
        if let Ok(size_str) = std::str::from_utf8(size_bytes)
            && let Ok(expected_size) = size_str.parse::<usize>()
        {
            // Validate: size must match actual content length
            let actual_size = blob_data.len() - (null_pos + 1);
            if expected_size == actual_size {
                // Perfect match, return stripped content
                return &blob_data[null_pos + 1..];
            }
        }

        // Validation failed, fallback to original content
        tracing::warn!(
            "Blob data starts with 'blob ' but validation failed, treating as raw content. \
             This may indicate data corruption or API contract violation."
        );
    }
    // No null byte found, fallback to original content

    // Not Git format, or validation failed, return original content
    blob_data
}

/// Internal helper function that computes both content hashes and blob IDs.
///
/// This function retrieves blob IDs for files, extracts raw content from Git blobs,
/// and calculates SHA-1 hash of the raw content. Returns both the content hashes
/// and the blob IDs to avoid duplicate queries.
///
/// # Arguments
/// * `handler` - API handler implementing ApiHandler trait
/// * `paths` - Slice of file paths to query
/// * `refs` - Optional commit hash or ref name
///
/// # Returns
/// Tuple of (content_hashes, blob_ids) where:
/// - content_hashes: HashMap mapping file paths to raw content SHA-1 hashes
/// - blob_ids: HashMap mapping file paths to ObjectHash blob IDs
///
/// Files not found will not be in the result
async fn get_files_content_hashes_internal<T: ApiHandler + ?Sized>(
    handler: &T,
    paths: &[PathBuf],
    refs: Option<&str>,
) -> Result<(HashMap<PathBuf, String>, HashMap<PathBuf, ObjectHash>), GitError> {
    if paths.is_empty() {
        return Ok((HashMap::new(), HashMap::new()));
    }

    // Get blob IDs for all files
    let blob_ids = get_files_blob_ids(handler, paths, refs).await?;

    if blob_ids.is_empty() {
        return Ok((HashMap::new(), HashMap::new()));
    }

    // Pre-allocate HashMap capacity to avoid reallocation during insertion
    let capacity = blob_ids.len();
    let mut content_hashes = HashMap::with_capacity(capacity);
    let mut successful_blob_ids = HashMap::with_capacity(capacity);

    // Batch read blob contents
    let max_concurrent = handler.get_context().get_recommended_batch_concurrency();
    let mut blob_stream = stream::iter(blob_ids.into_iter())
        .map(|(path, blob_hash)| {
            // Use into_iter to get ownership, no clone needed
            let blob_hash_str = blob_hash.to_string();
            async move {
                // Get blob data
                let result = handler.get_raw_blob_by_hash(&blob_hash_str).await;

                // If successful, immediately calculate hash
                match result {
                    Ok(blob_data) => {
                        // Extract raw content from Git blob format (returns slice, no copy)
                        let content_slice = extract_raw_content_from_blob(&blob_data);

                        // Calculate SHA-1 hash of raw content
                        let mut hasher = Sha1::new();
                        hasher.update(content_slice);
                        let hash = hex::encode(hasher.finalize());
                        (path, Ok((hash, blob_hash)))
                    }
                    Err(e) => (path, Err((blob_hash_str, e))),
                }
            }
        })
        .buffer_unordered(max_concurrent);

    // Process each result as it completes
    while let Some((path, result_item)) = blob_stream.next().await {
        match result_item {
            Ok((content_hash, blob_hash)) => {
                content_hashes.insert(path.clone(), content_hash);
                successful_blob_ids.insert(path, blob_hash);
            }
            Err((hash, e)) => {
                tracing::warn!("Failed to read blob {} for path {:?}: {}", hash, path, e);
                // Skip this file
            }
        }
    }

    Ok((content_hashes, successful_blob_ids))
}

/// Get content hashes (raw SHA-1) for multiple file paths in batch
///
/// This function retrieves blob IDs for files, extracts raw content from Git blobs,
/// and calculates SHA-1 hash of the raw content.
///
/// # Arguments
/// * `handler` - API handler implementing ApiHandler trait
/// * `paths` - Slice of file paths to query
/// * `refs` - Optional commit hash or ref name
///
/// # Returns
/// HashMap mapping file paths to raw content SHA-1 hashes
/// Files not found will not be in the result
pub async fn get_files_content_hashes<T: ApiHandler + ?Sized>(
    handler: &T,
    paths: &[PathBuf],
    refs: Option<&str>,
) -> Result<HashMap<PathBuf, String>, GitError> {
    let (content_hashes, _) = get_files_content_hashes_internal(handler, paths, refs).await?;
    Ok(content_hashes)
}

/// Get content hashes and blob IDs for multiple file paths in batch
///
/// This function retrieves blob IDs for files, extracts raw content from Git blobs,
/// and calculates SHA-1 hash of the raw content. Returns both values to avoid
/// duplicate queries when both are needed.
///
/// # Arguments
/// * `handler` - API handler implementing ApiHandler trait
/// * `paths` - Slice of file paths to query
/// * `refs` - Optional commit hash or ref name
///
/// # Returns
/// Tuple of (content_hashes, blob_ids) where:
/// - content_hashes: HashMap mapping file paths to raw content SHA-1 hashes
/// - blob_ids: HashMap mapping file paths to ObjectHash blob IDs
///
/// Files not found will not be in the result
pub async fn get_files_content_hashes_with_blob_ids<T: ApiHandler + ?Sized>(
    handler: &T,
    paths: &[PathBuf],
    refs: Option<&str>,
) -> Result<(HashMap<PathBuf, String>, HashMap<PathBuf, ObjectHash>), GitError> {
    get_files_content_hashes_internal(handler, paths, refs).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_raw_content_from_blob_valid_git_format() {
        // Test valid Git blob format
        let blob_data = b"blob 5\0hello";
        let result = extract_raw_content_from_blob(blob_data);
        assert_eq!(result, b"hello");
        // Verify it's a slice reference, not a copy
        assert_eq!(result.as_ptr(), blob_data[7..].as_ptr());
    }

    #[test]
    fn test_extract_raw_content_from_blob_raw_content() {
        // Test raw content (not Git format)
        let raw_data = b"hello world";
        let result = extract_raw_content_from_blob(raw_data);
        assert_eq!(result, raw_data);
        assert_eq!(result.as_ptr(), raw_data.as_ptr());
    }

    #[test]
    fn test_extract_raw_content_from_blob_size_mismatch() {
        // Test size mismatch (should fallback to original content)
        let invalid_blob = b"blob 10\0hello"; // size says 10, but content is 5 bytes
        let result = extract_raw_content_from_blob(invalid_blob);
        assert_eq!(result, invalid_blob); // Should return original content due to fallback
    }

    #[test]
    fn test_extract_raw_content_from_blob_non_digit_size() {
        // Test non-digit size (should fallback to original content)
        let invalid_size = b"blob abc\0hello";
        let result = extract_raw_content_from_blob(invalid_size);
        assert_eq!(result, invalid_size); // Should return original content due to fallback
    }

    #[test]
    fn test_extract_raw_content_from_blob_file_starting_with_blob() {
        // Test file starting with "blob " but not Git format (should return as-is)
        let regular_file = b"blob is a git object type";
        let result = extract_raw_content_from_blob(regular_file);
        assert_eq!(result, regular_file);
    }

    #[test]
    fn test_extract_raw_content_from_blob_empty_blob() {
        // Test empty blob
        let empty_blob = b"blob 0\0";
        let result = extract_raw_content_from_blob(empty_blob);
        assert_eq!(result, b"");
    }

    #[test]
    fn test_extract_raw_content_from_blob_large_blob() {
        // Test large blob
        let large_content = vec![b'a'; 1000];
        let mut large_blob = format!("blob {}\0", large_content.len()).into_bytes();
        large_blob.extend_from_slice(&large_content);
        let result = extract_raw_content_from_blob(&large_blob);
        assert_eq!(result, &large_content);
    }

    #[test]
    fn test_extract_raw_content_from_blob_missing_null() {
        // Test missing null byte (should fallback to original content)
        let no_null = b"blob 5hello";
        let result = extract_raw_content_from_blob(no_null);
        assert_eq!(result, no_null); // Should return original content due to fallback
    }

    #[test]
    fn test_extract_raw_content_from_blob_empty_size() {
        // Test empty size field (should fallback to original content)
        let empty_size = b"blob \0hello";
        let result = extract_raw_content_from_blob(empty_size);
        assert_eq!(result, empty_size); // Should return original content due to fallback
    }

    #[test]
    fn test_extract_raw_content_from_blob_fallback_behavior() {
        // Test fallback behavior: format validation fails should return original content
        // Size mismatch case
        let invalid_blob = b"blob 10\0hello"; // size says 10, but content is 5 bytes
        let result = extract_raw_content_from_blob(invalid_blob);
        assert_eq!(result, invalid_blob); // Should return original content

        // No null byte case
        let no_null = b"blob 5hello"; // no null byte
        let result = extract_raw_content_from_blob(no_null);
        assert_eq!(result, no_null); // Should return original content

        // Non-digit size case
        let invalid_size = b"blob abc\0hello";
        let result = extract_raw_content_from_blob(invalid_size);
        assert_eq!(result, invalid_size); // Should return original content
    }
}
