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
use tokio::sync::Mutex;

use crate::api_service::{
    ApiHandler, blob_ops,
    cache::{GitObjectCache, get_commit_from_cache, get_tree_from_cache},
    commit_ops, tree_ops,
};

/// Builds a mapping from each `TreeItem` under a given path to the earliest commit
/// in which that item appears in the repository history.
///
/// This function retrieves the tree corresponding to the given `path`, then for each entry
/// (file or subdirectory) in that tree, it traverses the commit history to determine the
/// earliest commit where the item first becomes reachable.
///
/// Internally, it leverages [`traverse_commit_history`] to perform the traversal
/// and uses a shared [`GitObjectCache`] to minimize redundant commit/tree lookups.
///
/// # Arguments
/// - `path`: The path to the target directory or subtree to analyze.
///
/// # Returns
/// - `Ok(HashMap<TreeItem, Option<Commit>>)` —  
///   A mapping from each tree item under the given path to the earliest commit containing it.  
///   If no tree is found for the path, an empty map is returned.
/// - `Err(GitError)` —  
///   If any Git object (tree or commit) cannot be read during traversal.
///
/// # Algorithm
/// 1. Retrieve the repository’s root commit (`HEAD` or equivalent).
/// 2. Resolve the tree at the given `path`.
/// 3. For each entry (`TreeItem`) in the resolved tree:
///    - Call [`traverse_commit_history`] to find the earliest commit
///      where that item exists under the specified path.
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
/// - [`traverse_commit_history`] — for the logic of commit traversal and earliest commit detection.
pub async fn item_to_commit_map<T: ApiHandler + ?Sized>(
    handler: &T,
    path: PathBuf,
) -> Result<HashMap<TreeItem, Option<Commit>>, GitError> {
    let cache = Arc::new(Mutex::new(GitObjectCache::default()));
    let root_commit = Arc::new(handler.get_root_commit().await);

    let Some(tree) = tree_ops::search_tree_by_path(handler, &path, None).await? else {
        return Ok(HashMap::new());
    };

    let mut result = HashMap::with_capacity(tree.tree_items.len());
    for item in tree.tree_items {
        let commit =
            traverse_commit_history(handler, &path, root_commit.clone(), &item, cache.clone())
                .await?;
        result.insert(item, Some(commit));
    }

    Ok(result)
}

/// Traverses the commit history starting from a given commit to find **the earliest commit**
/// in which the specified `TreeItem` exists under a given path.
///
/// This function performs a **breadth-first traversal (BFS)** of the commit graph, moving from
/// the given `start_commit` backwards through its parent commits.  
/// For each commit encountered, it loads the associated tree and checks whether the `TreeItem`
/// is reachable at the specified `path`.  
/// If the item is found in multiple commits, the function returns the one with the **earliest committer timestamp**.
///
/// # Arguments
/// - `path`: The path at which the target item is expected to be found.
/// - `start_commit`: The initial commit to begin traversal from (typically the head commit of the current branch).
/// - `search_item`: The `TreeItem` (file or directory) to locate in the commit history.
/// - `cache`: A shared, thread-safe [`GitObjectCache`] used to store and reuse loaded commits and trees.
///
/// # Returns
/// - `Ok(Commit)` — The earliest commit (by timestamp) where the target `TreeItem` exists under the given path.
/// - `Err(GitError)` — If reading commits, trees, or traversal encounters an error.
///
/// # Algorithm
/// 1. Initialize a queue with the `start_commit` and a visited set to prevent revisiting the same commits.
/// 2. For each commit dequeued:
///    - Retrieve its tree from the cache or repository.
///    - Check if the `TreeItem` is reachable at the provided `path`.
///    - If reachable:
///        - Add all unvisited parent commits to the queue for further traversal.
///        - Compare commit timestamps; update `target_commit` if this one is earlier.
/// 3. Continue until the queue is exhausted or all reachable commits are visited.
/// 4. Return the earliest commit containing the item.
///
/// # Notes
/// - This traversal is **timestamp-based**, not topologically sorted.
/// - It does **not** stop at the first found commit; it explores all reachable ancestors to ensure
///   the earliest match is found.
///
/// # Performance Considerations
/// - Uses [`Arc<Commit>`] and [`Arc<Mutex<GitObjectCache>>`] to avoid excessive cloning and repeated I/O.
/// - Breadth-first traversal ensures shallower commits are processed earlier, which helps avoid
///   excessive recursion depth compared to DFS.
/// - Cached trees and commits minimize repeated lookups.
///
/// # Locking
/// - The `cache` is shared across async calls with `Arc<Mutex<_>>`.
/// - Each lookup acquires the lock briefly to read or insert commits/trees into the cache.
///
pub async fn traverse_commit_history<T: ApiHandler + ?Sized>(
    handler: &T,
    path: &Path,
    start_commit: Arc<Commit>,
    search_item: &TreeItem,
    cache: Arc<Mutex<GitObjectCache>>,
) -> Result<Commit, GitError> {
    let mut target_commit = start_commit.clone();
    let mut visited = HashSet::new();
    let mut p_stack = VecDeque::new();

    visited.insert(start_commit.id);
    p_stack.push_back(start_commit);

    while let Some(commit) = p_stack.pop_front() {
        let root_tree = get_tree_from_cache(handler, commit.tree_id, &cache).await?;

        let reachable = reachable_in_tree(handler, root_tree, path, search_item, &cache).await?;

        if reachable {
            for &p_id in &commit.parent_commit_ids {
                if !visited.contains(&p_id) {
                    let p_commit = get_commit_from_cache(handler, p_id, &cache).await?;
                    p_stack.push_back(p_commit.clone());
                    visited.insert(p_id);
                }
            }
            if target_commit.committer.timestamp > commit.committer.timestamp {
                target_commit = commit.clone();
            }
        }
    }
    Ok((*target_commit).clone())
}

/// Determines whether a given `TreeItem` is reachable under a specified path
/// within a commit's root tree.
///
/// This function walks through the tree hierarchy following the given `path`
/// starting from the `root_tree`, resolving each subdirectory component
/// via the object cache until it reaches the final target tree.
///
/// Once the target directory tree is resolved, it checks whether the
/// specified `search_item` (file or subdirectory) exists directly within it.
///
/// # Arguments
/// - `root_tree`: The root tree object of a commit.
/// - `path`: The absolute or repository-relative path where the item should be searched.
/// - `search_item`: The `TreeItem` to search for (represents a file or subdirectory entry).
/// - `cache`: A shared [`GitObjectCache`] wrapped in `Arc<Mutex<_>>` to optimize tree lookups.
///
/// # Returns
/// - `Ok(true)` if the target `TreeItem` is found within the directory resolved from `path`.
/// - `Ok(false)` if the item is not present at that location.
/// - `Err(GitError)` if path resolution or tree lookup fails.
///
/// # Algorithm
/// 1. Convert the provided path to a repository-relative form via [`strip_relative`].
/// 2. Starting from the `root_tree`, iterate over each path component:
///    - For each directory name, locate its corresponding `TreeItem` entry.
///    - If found, load its associated subtree from the cache.
///    - If not found, return `Ok(false)` immediately.
/// 3. After reaching the target directory tree, check whether `search_item`
///    exists among its entries.
///
/// # Performance Notes
/// - Reuses cached trees via [`get_tree_from_cache`] to minimize I/O.
/// - Stops traversal early if any intermediate directory is missing.
///
/// # See Also
/// - [`traverse_commit_history`] — which uses this function during commit traversal.
/// - [`get_tree_from_cache`] — for efficient retrieval of tree objects.
async fn reachable_in_tree<T: ApiHandler + ?Sized>(
    handler: &T,
    root_tree: Arc<Tree>,
    path: &Path,
    search_item: &TreeItem,
    cache: &Arc<Mutex<GitObjectCache>>,
) -> Result<bool, GitError> {
    let relative_path = handler
        .strip_relative(path)
        .map_err(|e| GitError::CustomError(e.to_string()))?;
    let mut search_tree = root_tree;
    // first find search tree by path
    for component in relative_path.components() {
        // root tree already found
        if component != Component::RootDir {
            let target_name = component.as_os_str().to_str().unwrap();
            let search_res = search_tree
                .tree_items
                .iter()
                .find(|x| x.name == target_name);
            if let Some(search_res) = search_res {
                search_tree = get_tree_from_cache(handler, search_res.id, cache).await?;
            } else {
                return Ok(false);
            }
        }
    }
    // check item exist under search tree
    if search_tree.tree_items.iter().any(|x| x == search_item) {
        return Ok(true);
    }
    Ok(false)
}

/// Refs-aware version; default fallback to refs-unaware implementation
pub async fn item_to_commit_map_with_refs<T: ApiHandler + ?Sized>(
    handler: &T,
    path: PathBuf,
    _refs: Option<&str>,
) -> Result<HashMap<TreeItem, Option<Commit>>, GitError> {
    item_to_commit_map(handler, path).await
}

/// Precise algorithm: walk commit history from HEAD and return the newest commit
/// where the file blob at `path` differs from its parent (or was added).
/// Returns Ok(Some(commit)) on success, Ok(None) if file not found at HEAD.
pub async fn resolve_latest_commit_for_file_path<T: ApiHandler + ?Sized>(
    handler: &T,
    path: &Path,
) -> Result<Option<Commit>, GitError> {
    // Ensure file exists at HEAD and capture its blob id
    let head_tree = handler.get_root_tree(None).await?;
    let head_commit =
        commit_ops::get_tree_relate_commit(handler, head_tree.id, PathBuf::from("/")).await?;

    let current_blob =
        blob_ops::get_file_blob_id(handler, path, Some(&head_commit.id.to_string())).await?;
    let Some(mut curr_blob) = current_blob else {
        return Ok(None);
    };

    let mut curr_commit = head_commit.clone();
    // Safety guard to avoid pathological loops on very deep histories
    let mut steps: u32 = 0;
    const MAX_STEPS: u32 = 10_000;

    loop {
        steps += 1;
        if steps > MAX_STEPS {
            // Fallback: give up and return HEAD commit to avoid timeouts
            tracing::warn!(
                "resolve_latest_commit_for_file_path hit MAX_STEPS for path: {:?}",
                path
            );
            return Ok(Some(curr_commit));
        }

        // Single-parent traversal (our commits are linear fast-forward in Mono)
        let parent_id_opt = curr_commit.parent_commit_ids.first().cloned();
        let Some(parent_id) = parent_id_opt else {
            // Reached root of history; current commit introduced the file or first reference
            return Ok(Some(curr_commit));
        };

        let parent_commit = handler.get_commit_by_hash(&parent_id.to_string()).await;
        let parent_blob =
            blob_ops::get_file_blob_id(handler, path, Some(&parent_commit.id.to_string())).await?;

        if parent_blob.is_none() {
            // File did not exist in parent, so current commit added it
            return Ok(Some(curr_commit));
        }
        let p_blob = parent_blob.unwrap();
        if p_blob != curr_blob {
            // Blob changed between parent and current -> current touched the path
            return Ok(Some(curr_commit));
        }
        // Otherwise continue walking back
        curr_commit = parent_commit;
        curr_blob = p_blob;
    }
}
