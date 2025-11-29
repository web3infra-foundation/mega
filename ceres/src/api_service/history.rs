use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use git_internal::{
    errors::GitError,
    internal::object::{
        commit::Commit,
        tree::{Tree, TreeItem},
    },
};

use crate::api_service::ApiHandler;

/// Builds a mapping from each `TreeItem` under a given path to the commit
/// where that item was last modified.
///
/// This function retrieves the tree corresponding to the given `path`, then for each entry
/// (file or subdirectory) in that tree, it traverses the commit history backwards to find
/// the most recent commit where the item's hash changed (i.e., was modified).
///
/// If a `ref` (reference, such as a tag) is provided, traversal starts from the commit
/// that the reference points to. Otherwise, it starts from the repository's root commit.
///
/// Internally, it leverages [`traverse_commit_history_for_last_modification`] to perform
/// the traversal and uses a shared [`GitObjectCache`] to minimize redundant commit/tree lookups.
///
/// # Arguments
/// - `path`: The path to the target directory or subtree to analyze.
/// - `reference`: Optional reference (e.g., tag name) to start traversal from.
///   If `None`, starts from the repository's root commit.
///
/// # Returns
/// - `Ok(HashMap<TreeItem, Option<Commit>>)` —  
///   A mapping from each tree item under the given path to the commit where it was last modified.  
///   If no tree is found for the path, an empty map is returned.
/// - `Err(GitError)` —  
///   If any Git object (tree or commit) cannot be read during traversal, or if the reference
///   cannot be resolved.
///
/// # Algorithm
/// 1. Resolve the starting commit (from `reference` if provided, otherwise root commit).
/// 2. Resolve the tree at the given `path` at the starting commit.
/// 3. For each entry (`TreeItem`) in the resolved tree:
///    - Call [`traverse_commit_history_for_last_modification`] to find the most recent commit
///      where that item's hash changed, starting from the resolved commit and traversing backwards.
///    - Store the mapping in the result map.
/// 4. Return the complete mapping.
///
/// # Performance Notes
/// - Uses a shared `Arc<Mutex<GitObjectCache>>` for caching to avoid redundant
///   lookups of commits and trees.
/// - Each item is processed sequentially, so total runtime scales linearly
///   with the number of items in the directory.
///
/// # See Also
/// - [`traverse_commit_history_for_last_modification`] — for the logic of commit traversal
///   and last modification detection.
pub async fn item_to_commit_map<T: ApiHandler + ?Sized>(
    handler: &T,
    path: PathBuf,
    reference: Option<&str>,
) -> Result<HashMap<TreeItem, Option<Commit>>, GitError> {
    // Resolve the starting commit using unified refs resolution logic
    let start_commit_arc =
        crate::api_service::commit_ops::resolve_start_commit(handler, reference).await?;

    // Get the tree at the specified path from the resolved start commit
    // Use the start commit's tree to ensure consistency
    let start_tree = handler
        .get_tree_by_hash(&start_commit_arc.tree_id.to_string())
        .await?;

    // Navigate to the target path within the start commit's tree
    let Some(tree) = navigate_to_tree(handler, Arc::new(start_tree), &path).await? else {
        return Ok(HashMap::new());
    };

    // For each item in the tree, traverse commit history to find its last modification
    let tree_items = tree.tree_items.clone();
    let mut result = HashMap::with_capacity(tree_items.len());

    for item in tree_items {
        let commit = traverse_commit_history_for_last_modification(
            handler,
            &path,
            start_commit_arc.clone(),
            &item,
        )
        .await?;
        result.insert(item, Some(commit));
    }

    Ok(result)
}

/// Navigates through the tree hierarchy following a given path, starting from a root tree.
/// Returns the target tree at the end of the path, or None if the path doesn't exist.
///
/// # Arguments
/// - `handler`: The API handler for Git operations.
/// - `root_tree`: The root tree to start navigation from.
/// - `path`: The path to navigate to.
/// - `cache`: A shared cache for tree lookups.
///
/// # Returns
/// - `Ok(Some(Arc<Tree>))` if the path exists and the target tree is found.
/// - `Ok(None)` if any component in the path doesn't exist.
/// - `Err(GitError)` if path resolution or tree lookup fails.
async fn navigate_to_tree<T: ApiHandler + ?Sized>(
    handler: &T,
    root_tree: Arc<Tree>,
    path: &Path,
) -> Result<Option<Arc<Tree>>, GitError> {
    let relative_path = handler
        .strip_relative(path)
        .map_err(|e| GitError::CustomError(e.to_string()))?;
    let mut search_tree = root_tree;
    for component in relative_path.components() {
        if component != Component::RootDir {
            let target_name = component.as_os_str().to_str().ok_or_else(|| {
                GitError::CustomError(format!(
                    "Path component is not valid UTF-8: {:?}",
                    component
                ))
            })?;

            let search_res = search_tree
                .tree_items
                .iter()
                .find(|x| x.name == target_name);

            if let Some(search_res) = search_res {
                // Only descend into tree entries; hitting a blob here means the path
                // is pointing to a file where a directory/tree is expected.
                if !search_res.is_tree() {
                    return Ok(None);
                }
                let tree_id = search_res.id;
                search_tree = handler
                    .object_cache()
                    .get_tree(tree_id, |tree_id| async move {
                        handler.get_tree_by_hash(&tree_id.to_string()).await
                    })
                    .await?;
            } else {
                return Ok(None);
            }
        }
    }
    Ok(Some(search_tree))
}

/// Resolves the last modification commit for a file or directory given its full path.
///
/// This is a convenience wrapper around [`traverse_commit_history_for_last_modification`]
/// that handles path parsing and `TreeItem` lookup for complete file or directory paths.
///
/// # Arguments
/// - `handler`: The API handler providing Git operations
/// - `full_path`: Full path to the file or directory (e.g., `/src/main.rs` or `/src`)
/// - `start_commit`: Starting commit for traversal (typically HEAD or from a ref)
/// - `cache`: Shared cache for commits and trees to optimize performance
///
/// # Returns
/// - `Ok(Commit)`: The commit where the file/directory was last modified
/// - `Err(GitError)`: If the path doesn't exist, is invalid, or traversal fails
///
/// # Examples
/// ```ignore
/// let start = Arc::new(handler.get_root_commit().await);
///
/// // Find last modification of a file
/// let commit = resolve_last_modification_by_path(
///     handler,
///     Path::new("/src/main.rs"),
///     start,
///     cache
/// ).await?;
/// ```
///
/// # Implementation Details
/// This function:
/// 1. Splits the path into parent directory and item name
/// 2. Navigates to the parent directory in the start commit
/// 3. Finds the corresponding `TreeItem` for the file/directory
/// 4. Calls [`traverse_commit_history_for_last_modification`] with the extracted item
pub async fn resolve_last_modification_by_path<T: ApiHandler + ?Sized>(
    handler: &T,
    full_path: &Path,
    start_commit: Arc<Commit>,
) -> Result<Commit, GitError> {
    // 1. Extract the item name from the path
    let item_name = full_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            GitError::CustomError(format!(
                "Invalid path: cannot extract name from '{}'",
                full_path.display()
            ))
        })?
        .to_string();

    // 2. Get the parent directory path
    let parent_path = full_path.parent().ok_or_else(|| {
        GitError::CustomError(format!(
            "Path '{}' has no parent directory",
            full_path.display()
        ))
    })?;

    // 3. Navigate to the parent directory in the start commit
    let tree_id = start_commit.tree_id;
    let start_tree = handler
        .object_cache()
        .get_tree(tree_id, |tree_id| async move {
            handler.get_tree_by_hash(&tree_id.to_string()).await
        })
        .await?;
    let Some(parent_tree) = navigate_to_tree(handler, start_tree, parent_path).await? else {
        return Err(GitError::CustomError(format!(
            "Parent directory '{}' not found in commit {}",
            parent_path.display(),
            start_commit.id
        )));
    };

    // 4. Find the TreeItem for the target file/directory
    let Some(item) = parent_tree
        .tree_items
        .iter()
        .find(|x| x.name == item_name)
        .cloned()
    else {
        return Err(GitError::CustomError(format!(
            "Item '{}' not found in directory '{}'",
            item_name,
            parent_path.display()
        )));
    };

    // 5. Call the core last-modification traversal logic
    traverse_commit_history_for_last_modification(handler, parent_path, start_commit, &item).await
}

/// Traverses commit history from a starting commit to find the last commit
/// where a specific `TreeItem`'s hash changed between that commit and its parent.
///
/// This function walks the parent chain starting from `start_commit` and returns
/// the first commit where the item's hash at `path` differs from its direct
/// parent's hash (or where the item did not exist in the parent). This matches
/// the "last modification" semantics used by tools like `git log -1 <path>`
/// or `git blame`, which identify the most recent commit where a file or
/// directory was changed.
///
/// # Arguments
/// - `path`: The directory path under which the target item is expected.
/// - `start_commit`: The commit to begin traversal from (e.g., HEAD or a tag commit).
/// - `search_item`: The `TreeItem` (file or directory) to track.
/// - `cache`: A shared cache for commits and trees.
///
/// # Returns
/// - `Ok(Commit)` — The commit where the item was last modified, or `start_commit` if unchanged.
/// - `Err(GitError)` — If the path or item does not exist in `start_commit`, or if traversal fails.
pub async fn traverse_commit_history_for_last_modification<T: ApiHandler + ?Sized>(
    handler: &T,
    path: &Path,
    start_commit: Arc<Commit>,
    search_item: &TreeItem,
) -> Result<Commit, GitError> {
    // Resolve the item's hash in the starting commit at the given path
    let tree_id = start_commit.tree_id;
    let start_tree = handler
        .object_cache()
        .get_tree(tree_id, |tree_id| async move {
            handler.get_tree_by_hash(&tree_id.to_string()).await
        })
        .await?;

    let Some(target_tree) = navigate_to_tree(handler, start_tree, path).await? else {
        // Path does not exist at start_commit; return an error
        return Err(GitError::CustomError(format!(
            "[code:404] Path not found: {:?} at commit {}",
            path, start_commit.id
        )));
    };

    let Some(current_hash) = target_tree
        .tree_items
        .iter()
        .find(|x| x.name == search_item.name && x.mode == search_item.mode)
        .map(|x| x.id)
    else {
        // Item does not exist at start_commit; return an error
        return Err(GitError::CustomError(format!(
            "[code:404] Item '{}' does not exist at path '{}' in commit {}",
            search_item.name,
            path.display(),
            start_commit.id
        )));
    };

    // Walk commit history using a queue (BFS over the commit graph) while
    // always comparing a commit's hash at `path` with its direct parent's
    // hash. The first commit we encounter where the hash differs (or the
    // item/path is missing in the parent) is treated as the last modification,
    let mut visited = HashSet::new();
    let mut commit_queue: VecDeque<(Arc<Commit>, git_internal::hash::SHA1)> = VecDeque::new();

    visited.insert(start_commit.id);
    commit_queue.push_back((start_commit.clone(), current_hash));

    // Safety limit to prevent unbounded memory usage in complex histories
    const MAX_ITERATIONS: usize = 10_000;
    let mut iterations = 0;

    while let Some((commit, commit_hash)) = commit_queue.pop_front() {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            tracing::warn!(
                "Exceeded maximum iterations ({}) in traverse_commit_history_for_last_modification for path: {:?}",
                MAX_ITERATIONS,
                path
            );
            return Ok((*start_commit).clone());
        }
        // No parents: this commit introduced the item or is the earliest
        // reference we can see. Treat it as the last modification.
        if commit.parent_commit_ids.is_empty() {
            return Ok((*commit).clone());
        }

        // Collect information about all parents first to properly handle merge commits
        let mut parents_with_matching_hash = Vec::new();

        for &parent_id in &commit.parent_commit_ids {
            let parent_commit = handler
                .object_cache()
                .get_commit(parent_id, |parent_id| async move {
                    handler.get_commit_by_hash(&parent_id.to_string()).await
                })
                .await?;
            let tree_id = parent_commit.tree_id;
            let parent_tree = handler
                .object_cache()
                .get_tree(tree_id, |tree_id| async move {
                    handler.get_tree_by_hash(&tree_id.to_string()).await
                })
                .await?;

            // Navigate to the target directory in the parent commit
            let parent_target_tree_opt = navigate_to_tree(handler, parent_tree, path).await?;

            // Check if parent has the same item with the same hash
            let parent_item_hash = if let Some(parent_target_tree) = parent_target_tree_opt {
                parent_target_tree
                    .tree_items
                    .iter()
                    .find(|x| x.name == search_item.name && x.mode == search_item.mode)
                    .map(|x| x.id)
            } else {
                None
            };

            // if parent has identical content, it's a candidate for traversal
            if parent_item_hash == Some(commit_hash) {
                parents_with_matching_hash.push((parent_commit, commit_hash));
            }
        }

        // Decision logic:
        // 1. If any parent has matching hash, continue traversing only those parents
        //    (the commit inherited content from them, not a real modification)
        // 2. If NO parent has matching hash (all different or missing), then this
        //    commit is the last modification
        if !parents_with_matching_hash.is_empty() {
            // Commit inherited content from at least one parent
            // Continue traversing only the parents with matching hash
            for (parent_commit, p_hash) in parents_with_matching_hash {
                // Only queue if not already visited
                if !visited.contains(&parent_commit.id) {
                    visited.insert(parent_commit.id);
                    commit_queue.push_back((parent_commit, p_hash));
                }
            }
        } else {
            return Ok((*commit).clone());
        }
    }

    // Fallback: Queue exhausted without finding a modification.
    // This can occur if all commits back to the root have matching hashes,
    // meaning the file has never been modified since its introduction.
    // In this case, start_commit is the "last" (and only) modification point.
    Ok((*start_commit).clone())
}
