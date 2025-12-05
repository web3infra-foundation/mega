use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use git_internal::{
    errors::GitError,
    hash::SHA1,
    internal::object::{
        commit::Commit,
        tree::{Tree, TreeItem, TreeItemMode},
    },
};

use crate::api_service::ApiHandler;

const MAX_ITERATIONS: usize = 10_000;

/// Parent commit info with pre-computed item hash map for batch lookup.
struct ParentInfo {
    commit: Arc<Commit>,
    /// Map of item name -> (hash, mode) for quick TREESAME check
    item_hashes: HashMap<String, (SHA1, TreeItemMode)>,
}

/// State for tracking a single item's traversal through commit history.
struct ItemTraversalState {
    /// The tree item being tracked
    item: TreeItem,
    /// BFS queue: (commit_id, item_hash_at_that_commit)
    queue: VecDeque<(SHA1, SHA1)>,
    /// Set of visited commit IDs to avoid cycles
    visited: HashSet<SHA1>,
    /// Whether this item's last modification has been determined
    determined: bool,
    /// The commit where this item was last modified (set when determined)
    result_commit: Option<Commit>,
}

/// Builds a mapping from each `TreeItem` under a given path to the commit
/// where that item was last modified.
///
/// This function retrieves the tree corresponding to the given `path`, then uses a
/// batch traversal algorithm to find the most recent commit where each item's hash
/// changed (i.e., was modified).
///
/// If a `ref` (reference, such as a tag) is provided, traversal starts from the commit
/// that the reference points to. Otherwise, it starts from the repository's root commit.
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
pub async fn item_to_commit_map<T: ApiHandler + ?Sized>(
    handler: &T,
    path: PathBuf,
    reference: Option<&str>,
) -> Result<HashMap<TreeItem, Option<Commit>>, GitError> {
    // Resolve the starting commit
    let start_commit_arc =
        crate::api_service::commit_ops::resolve_start_commit(handler, reference).await?;

    // Get the tree at the specified path from the resolved start commit
    let start_tree = handler
        .get_tree_by_hash(&start_commit_arc.tree_id.to_string())
        .await?;

    // Navigate to the target path within the start commit's tree
    let Some(tree) = navigate_to_tree(handler, Arc::new(start_tree), &path).await? else {
        return Ok(HashMap::new());
    };

    let tree_items = tree.tree_items.clone();
    if tree_items.is_empty() {
        return Ok(HashMap::new());
    }

    // Build items list with their current hashes
    let items: Vec<(TreeItem, SHA1)> = tree_items
        .into_iter()
        .map(|item| {
            let hash = item.id;
            (item, hash)
        })
        .collect();

    // Call core traversal function
    let result =
        traverse_items_for_last_modification(handler, &path, start_commit_arc, items).await?;

    // Convert to Option<Commit> format
    Ok(result.into_iter().map(|(k, v)| (k, Some(v))).collect())
}

/// Core traversal function that finds the last modification commit for each item.
///
/// This function uses BFS with TREESAME pruning to traverse commit history and find
/// where each item was last modified. It maintains caches for commits, trees, and
/// intermediate path navigation to optimize performance.
///
/// # Arguments
/// - `handler`: The API handler for Git operations.
/// - `path`: The directory path where items are located.
/// - `start_commit`: The commit to begin traversal from.
/// - `items`: List of (TreeItem, current_hash) pairs to track.
///
/// # Returns
/// - `Ok(HashMap<TreeItem, Commit>)` — A mapping from each TreeItem to the commit where it was last modified.
///   If traversal reaches `MAX_ITERATIONS` without determining all items, undetermined items
///   will fallback to `start_commit`.
/// - `Err(GitError)` — If any Git object (tree or commit) cannot be loaded during traversal.
async fn traverse_items_for_last_modification<T: ApiHandler + ?Sized>(
    handler: &T,
    path: &Path,
    start_commit: Arc<Commit>,
    items: Vec<(TreeItem, SHA1)>,
) -> Result<HashMap<TreeItem, Commit>, GitError> {
    let item_count = items.len();
    if item_count == 0 {
        return Ok(HashMap::new());
    }

    // Initialize traversal state for each item
    let mut item_states: Vec<ItemTraversalState> = items
        .into_iter()
        .map(|(item, hash)| {
            let mut queue = VecDeque::new();
            let mut visited = HashSet::new();
            queue.push_back((start_commit.id, hash));
            visited.insert(start_commit.id);
            ItemTraversalState {
                item,
                queue,
                visited,
                determined: false,
                result_commit: None,
            }
        })
        .collect();

    let mut pending_count = item_count;

    // Cache for commits we've loaded
    let mut commit_cache: HashMap<SHA1, Arc<Commit>> = HashMap::new();
    commit_cache.insert(start_commit.id, start_commit.clone());

    // Cache for tree_id -> item_hashes at the target path
    let mut tree_items_cache: HashMap<SHA1, HashMap<String, (SHA1, TreeItemMode)>> = HashMap::new();

    // Cache for intermediate directory trees during path navigation
    let mut path_tree_cache: HashMap<SHA1, Arc<Tree>> = HashMap::new();

    let mut iteration = 0;

    while pending_count > 0 {
        iteration += 1;
        if iteration > MAX_ITERATIONS {
            tracing::warn!(
                "[HISTORY] Exceeded max iterations ({}) for path {:?}, {} items still pending",
                MAX_ITERATIONS,
                path,
                pending_count
            );
            break;
        }

        // Collect current batch from each undetermined item's queue
        let mut current_batch: Vec<(usize, SHA1, SHA1)> = Vec::new();
        for (idx, state) in item_states.iter_mut().enumerate() {
            if !state.determined
                && let Some((commit_id, item_hash)) = state.queue.pop_front()
            {
                current_batch.push((idx, commit_id, item_hash));
            }
        }

        if current_batch.is_empty() {
            break;
        }

        // Group by commit_id to share parent loading
        let mut commit_groups: HashMap<SHA1, Vec<(usize, SHA1)>> = HashMap::new();
        for (idx, commit_id, item_hash) in current_batch {
            commit_groups
                .entry(commit_id)
                .or_default()
                .push((idx, item_hash));
        }

        // Process each commit group
        for (commit_id, items_data) in commit_groups {
            // Load commit from cache or database
            let commit = if let Some(c) = commit_cache.get(&commit_id) {
                c.clone()
            } else {
                let c = handler
                    .object_cache()
                    .get_commit(commit_id, |id| async move {
                        handler.get_commit_by_hash(&id.to_string()).await
                    })
                    .await?;
                commit_cache.insert(commit_id, c.clone());
                c
            };

            // If no parents, all items in this group were introduced in this commit
            if commit.parent_commit_ids.is_empty() {
                for (item_idx, _) in items_data {
                    if !item_states[item_idx].determined {
                        item_states[item_idx].determined = true;
                        item_states[item_idx].result_commit = Some((*commit).clone());
                        pending_count -= 1;
                    }
                }
                continue;
            }

            // Load parent commits and their trees
            let mut parent_data: Vec<ParentInfo> = Vec::new();
            for &parent_id in &commit.parent_commit_ids {
                let parent_commit = if let Some(c) = commit_cache.get(&parent_id) {
                    c.clone()
                } else {
                    let c = handler
                        .object_cache()
                        .get_commit(parent_id, |id| async move {
                            handler.get_commit_by_hash(&id.to_string()).await
                        })
                        .await?;
                    commit_cache.insert(parent_id, c.clone());
                    c
                };

                // Get item hashes at target path from cache or load
                let parent_item_hashes =
                    if let Some(cached) = tree_items_cache.get(&parent_commit.tree_id) {
                        cached.clone()
                    } else {
                        let parent_tree = handler
                            .object_cache()
                            .get_tree(parent_commit.tree_id, |id| async move {
                                handler.get_tree_by_hash(&id.to_string()).await
                            })
                            .await?;

                        let parent_target_tree_opt = navigate_to_tree_with_cache(
                            handler,
                            parent_tree,
                            path,
                            &mut path_tree_cache,
                        )
                        .await?;

                        // Build hash map for TREESAME check
                        let item_hashes: HashMap<String, (SHA1, TreeItemMode)> =
                            if let Some(ref t) = parent_target_tree_opt {
                                t.tree_items
                                    .iter()
                                    .map(|item| (item.name.clone(), (item.id, item.mode)))
                                    .collect()
                            } else {
                                HashMap::new()
                            };

                        tree_items_cache.insert(parent_commit.tree_id, item_hashes.clone());
                        item_hashes
                    };

                parent_data.push(ParentInfo {
                    commit: parent_commit,
                    item_hashes: parent_item_hashes,
                });
            }

            // Check TREESAME for each item in this group
            for (item_idx, item_hash) in items_data {
                let state = &mut item_states[item_idx];
                if state.determined {
                    continue;
                }

                let item_name = &state.item.name;
                let item_mode = state.item.mode;

                // Collect all TREESAME parents
                let mut treesame_parents: Vec<(SHA1, SHA1)> = Vec::new();
                for parent in &parent_data {
                    if let Some(&(parent_hash, parent_mode)) = parent.item_hashes.get(item_name)
                        && parent_hash == item_hash
                        && parent_mode == item_mode
                    {
                        treesame_parents.push((parent.commit.id, parent_hash));
                    }
                }

                if treesame_parents.is_empty() {
                    // No TREESAME parent: this commit modified the item
                    state.determined = true;
                    state.result_commit = Some((*commit).clone());
                    pending_count -= 1;
                } else {
                    // Enqueue TREESAME parents for further traversal
                    for (parent_id, parent_hash) in treesame_parents {
                        if !state.visited.contains(&parent_id) {
                            state.visited.insert(parent_id);
                            state.queue.push_back((parent_id, parent_hash));
                        }
                    }
                }
            }
        }
    }

    // Build the result map
    let mut result = HashMap::with_capacity(item_count);
    for state in item_states {
        let commit = state
            .result_commit
            .unwrap_or_else(|| (*start_commit).clone());
        result.insert(state.item, commit);
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
///
/// # Returns
/// - `Ok(Some(Arc<Tree>))` if the path exists and the target tree is found.
/// - `Ok(None)` if any component in the path doesn't exist.
/// - `Err(GitError)` if path resolution or tree lookup fails.
pub async fn navigate_to_tree<T: ApiHandler + ?Sized>(
    handler: &T,
    root_tree: Arc<Tree>,
    path: &Path,
) -> Result<Option<Arc<Tree>>, GitError> {
    navigate_to_tree_with_cache(handler, root_tree, path, &mut HashMap::new()).await
}

/// Navigates through tree hierarchy with a shared cache for intermediate trees.
/// Used internally by `item_to_commit_map` to avoid repeated lookups of the same
/// directory trees (e.g., "/third-party") across different root trees.
async fn navigate_to_tree_with_cache<T: ApiHandler + ?Sized>(
    handler: &T,
    root_tree: Arc<Tree>,
    path: &Path,
    tree_cache: &mut HashMap<SHA1, Arc<Tree>>,
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
                if !search_res.is_tree() {
                    return Ok(None);
                }
                let tree_id = search_res.id;
                search_tree = if let Some(cached) = tree_cache.get(&tree_id) {
                    cached.clone()
                } else {
                    let tree = handler
                        .object_cache()
                        .get_tree(tree_id, |tree_id| async move {
                            handler.get_tree_by_hash(&tree_id.to_string()).await
                        })
                        .await?;
                    tree_cache.insert(tree_id, tree.clone());
                    tree
                };
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
/// - `handler`: The API handler for Git operations.
/// - `path`: The directory path under which the target item is expected.
/// - `start_commit`: The commit to begin traversal from (e.g., HEAD or a tag commit).
/// - `search_item`: The `TreeItem` (file or directory) to track.
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
    // Verify the item exists in start_commit and get its current hash
    let tree_id = start_commit.tree_id;
    let start_tree = handler
        .object_cache()
        .get_tree(tree_id, |tree_id| async move {
            handler.get_tree_by_hash(&tree_id.to_string()).await
        })
        .await?;

    let Some(target_tree) = navigate_to_tree(handler, start_tree, path).await? else {
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
        return Err(GitError::CustomError(format!(
            "[code:404] Item '{}' does not exist at path '{}' in commit {}",
            search_item.name,
            path.display(),
            start_commit.id
        )));
    };

    // Call core traversal function with single item
    let items = vec![(search_item.clone(), current_hash)];
    let result = traverse_items_for_last_modification(handler, path, start_commit, items).await?;

    // Extract the single result
    result.into_values().next().ok_or_else(|| {
        GitError::CustomError(format!(
            "[code:500] Internal error: traversal returned no results for item '{}'",
            search_item.name
        ))
    })
}
