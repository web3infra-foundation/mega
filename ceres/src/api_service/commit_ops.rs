use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};

use api_model::{
    common::{CommonPage, Pagination},
    git::commit::LatestCommitInfo,
};
use git_internal::{
    DiffItem,
    errors::GitError,
    hash::ObjectHash,
    internal::object::{
        commit::Commit,
        tree::{TreeItem, TreeItemMode},
    },
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::{
    api_service::{ApiHandler, history, tree_ops},
    model::{
        change_list::{DiffItemSchema, MuiTreeNode},
        commit::{CommitFilesChangedPage, CommitSummary, GpgStatus},
        git::{CommitBindingInfo, LatestCommitInfoWrapper},
    },
};

/// Compute GPG signature verification status for a commit.
///
/// This function checks whether a commit has a valid GPG signature by:
/// 1. Checking Redis cache for previously computed status
/// 2. Extracting the GPG signature from commit content
/// 3. Verifying the signature against stored GPG keys for the committer
///
/// # Arguments
/// * `handler` - API handler providing storage and cache access
/// * `commit` - The commit to verify
/// * `committer` - Username of the committer for key lookup
///
/// # Returns
/// * `GpgStatus::Verified` - Commit has a valid signature
/// * `GpgStatus::Unverified` - Commit has a signature but verification failed
/// * `GpgStatus::NoSignature` - Commit is not signed
///
/// # Cache
/// Results are cached in Redis with a 600-second TTL to avoid redundant verification.
async fn compute_gpg_status_for_commit<T: ApiHandler + ?Sized>(
    handler: &T,
    commit: &Commit,
    committer: &str,
) -> GpgStatus {
    // Cache key per commit
    let cache_key = format!(
        "{}:gpg_status:v1:sha={}",
        handler.object_cache().prefix,
        commit.id
    );

    // Try cache first
    let mut conn = handler.object_cache().connection.clone();
    if let Ok(Some(json)) = conn.get::<_, Option<String>>(&cache_key).await
        && let Ok(status) = serde_json::from_str::<GpgStatus>(&json)
    {
        let _ = conn.expire::<_, ()>(&cache_key, 600).await;
        return status;
    }

    let content = &commit.message;
    let status = {
        let (_commit_msg, sig_opt) =
            crate::merge_checker::gpg_signature_checker::extract_from_commit_content(content);
        if sig_opt.is_none() {
            GpgStatus::NoSignature
        } else {
            let storage = handler.get_context();
            let checker = crate::merge_checker::gpg_signature_checker::GpgSignatureChecker {
                storage: Arc::new(storage),
            };
            match checker
                .verify_commit_gpg_signature(content, committer.to_string())
                .await
            {
                Ok(()) => GpgStatus::Verified,
                Err(e) => {
                    tracing::debug!("GPG verification failed for commit {}: {}", commit.id, e);
                    GpgStatus::Unverified
                }
            }
        }
    };

    // Save to cache
    match serde_json::to_string(&status) {
        Ok(json) => {
            if let Err(e) = conn.set_ex::<_, _, ()>(&cache_key, json, 600).await {
                tracing::warn!("Failed to cache GPG status for {}: {}", cache_key, e);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to serialize GPG status for {}: {}", cache_key, e);
        }
    }
    status
}

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
            let mut wrapper: LatestCommitInfoWrapper = (*start_commit).clone().into();

            // Apply username binding if available
            apply_username_binding(handler, &start_commit.id.to_string(), &mut wrapper).await;

            return Ok(wrapper.0);
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

        let mut wrapper: LatestCommitInfoWrapper = commit.clone().into();

        // Apply username binding if available
        apply_username_binding(handler, &commit.id.to_string(), &mut wrapper).await;

        return Ok(wrapper.0);
    }

    // 2) If not a directory, try as file path
    // Use unified last-modification logic
    match history::resolve_last_modification_by_path(handler, &path, start_commit).await {
        Ok(commit) => {
            let mut wrapper: LatestCommitInfoWrapper = commit.clone().into();

            // Apply username binding if available
            apply_username_binding(handler, &commit.id.to_string(), &mut wrapper).await;

            Ok(wrapper.0)
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
    commit_info: &mut LatestCommitInfoWrapper,
) {
    if let Ok(Some(binding)) = handler.build_commit_binding_info(commit_id).await
        && !binding.is_anonymous
        && let Some(username) = binding.matched_username
    {
        commit_info.0.author = username.clone();
        commit_info.0.committer = username;
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
/// It supports the `main` and `master` branch names (other branches not yet supported),
/// tags (with or without `refs/tags/` prefix), and commit SHAs.
///
/// # Arguments
/// - `handler`: The API handler providing Git operations
/// - `refs`: Optional reference string, which can be:
///   - `None` or empty string: returns root commit (HEAD)
///   - Branch name (`main` or `master` only; other branches not yet supported)
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
                "[code:404] Commit SHA '{}' not found",
                ref_str
            )));
        }

        return Ok(Arc::new(commit));
    }

    // Failed to resolve reference
    Err(GitError::CustomError(format!(
        "[code:400] Invalid reference '{}': only 'main'/'master' branches, tags, or commit SHAs are supported",
        ref_str
    )))
}

/// Compute the object hash (tree for directory, blob for file) at a path for a given commit.
/// Returns None if the path does not exist in that commit.
async fn compute_path_hash<T: ApiHandler + ?Sized>(
    handler: &T,
    commit: &Commit,
    path: &PathBuf,
) -> Result<Option<ObjectHash>, GitError> {
    let tree = handler
        .object_cache()
        .get_tree(commit.tree_id, |tree_id| async move {
            handler.get_tree_by_hash(&tree_id.to_string()).await
        })
        .await?;
    if path.as_os_str().is_empty() || path == &PathBuf::from("/") {
        return Ok(Some(tree.id));
    }
    let name = path
        .file_name()
        .ok_or_else(|| {
            GitError::CustomError(format!("Path has no filename component: {:?}", path))
        })?
        .to_str()
        .ok_or_else(|| {
            GitError::CustomError(format!("Path contains non-UTF-8 characters: {:?}", path))
        })?;
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("/"));
    let parent_opt = super::history::navigate_to_tree(handler, tree, parent).await?;
    if let Some(parent_tree) = parent_opt {
        Ok(parent_tree
            .tree_items
            .iter()
            .find(|x| x.name == name)
            .map(|x| x.id))
    } else {
        Ok(None)
    }
}

/// Traverse commit history: collect all reachable commits from a start ref, apply optional
/// path and author filters, then sort by committer timestamp descending (time priority).
pub async fn traverse_history_commits<T: ApiHandler + ?Sized>(
    handler: &T,
    start_refs: Option<&str>,
    path_filter: Option<&PathBuf>,
    author: Option<&str>,
    max_scan: usize,
) -> Result<Vec<Commit>, GitError> {
    // Resolve start commit from refs
    let start = resolve_start_commit(handler, start_refs).await?;

    // BFS to collect all reachable commits (avoid missing merge histories)
    let mut visited: HashSet<ObjectHash> = HashSet::new();
    let mut queue: VecDeque<Commit> = VecDeque::new();
    let mut all: Vec<Commit> = Vec::new();
    queue.push_back((*start).clone());

    while let Some(commit) = queue.pop_front() {
        if visited.contains(&commit.id) {
            continue;
        }
        visited.insert(commit.id);
        let parent_ids = commit.parent_commit_ids.clone();
        all.push(commit);

        for &pid in &parent_ids {
            let parent = handler
                .object_cache()
                .get_commit(pid, |id| async move {
                    handler.get_commit_by_hash(&id.to_string()).await
                })
                .await?;
            if !visited.contains(&parent.id) {
                queue.push_back((*parent).clone());
            }
        }
        if all.len() >= max_scan {
            break;
        }
    }

    // Optional path modification filter
    let matched_by_path: Vec<Commit> = if let Some(p_abs) = path_filter {
        let p_rel = handler
            .strip_relative(p_abs.as_path())
            .map_err(|e| GitError::CustomError(e.to_string()))?;
        let mut out = Vec::new();
        for c in &all {
            let curr = compute_path_hash(handler, c, &p_rel).await?;
            // For root commit (no parents): if the path exists, treat as changed
            if c.parent_commit_ids.is_empty() {
                if curr.is_some() {
                    out.push(c.clone());
                }
                continue;
            }
            // Git-like history simplification (path-limited):
            // A merge commit is considered a change for the path ONLY IF the
            // path's object differs from ALL parents (i.e., no parent has the
            // same tree/blob at that path). If any parent matches, the commit is
            // TREESAME for this path and should be omitted (default `git log <path>` behavior).
            let mut all_parents_differ = true;
            for &pid in &c.parent_commit_ids {
                let p = handler
                    .object_cache()
                    .get_commit(pid, |id| async move {
                        handler.get_commit_by_hash(&id.to_string()).await
                    })
                    .await?;
                let ph = compute_path_hash(handler, &p, &p_rel).await?;
                if curr == ph {
                    all_parents_differ = false;
                    break;
                }
            }
            let changed = all_parents_differ;
            if changed {
                out.push(c.clone());
            }
        }
        out
    } else {
        all
    };

    // Optional author filter (prefer bound username if present)
    let matched_by_author: Vec<Commit> =
        if let Some(a) = author.map(|s| s.trim()).filter(|t| !t.is_empty()) {
            let a_norm = a.to_lowercase();
            let mut out = Vec::new();
            for c in matched_by_path {
                let bound = build_commit_binding_info(handler, &c.id.to_string())
                    .await
                    .ok()
                    .flatten();
                let effective = bound
                    .filter(|b| !b.is_anonymous)
                    .and_then(|b| b.matched_username)
                    .unwrap_or_else(|| c.author.name.clone());
                if effective.to_lowercase() == a_norm {
                    out.push(c);
                }
            }
            out
        } else {
            matched_by_path
        };

    // Final sort: by committer timestamp descending
    let mut result = matched_by_author;
    result.sort_by_key(|b| std::cmp::Reverse(b.committer.timestamp));
    Ok(result)
}

/// Collect all blobs as (path, ObjectHash) pairs under a commit tree
pub async fn collect_commit_blobs<T: ApiHandler + ?Sized>(
    handler: &T,
    commit: &Commit,
) -> Result<Vec<(PathBuf, ObjectHash)>, GitError> {
    // Load the root tree for this commit and traverse it
    let root_tree = handler
        .get_tree_by_hash(&commit.tree_id.to_string())
        .await?;

    // Generic DFS traversal using handler.get_tree_by_hash for child trees
    let mut result: Vec<(PathBuf, ObjectHash)> = Vec::new();
    let mut stack: Vec<(PathBuf, git_internal::internal::object::tree::Tree)> =
        vec![(PathBuf::new(), root_tree)];
    while let Some((base, tree)) = stack.pop() {
        for item in tree.tree_items {
            let p = base.join(&item.name);
            if item.is_tree() {
                let child = handler.get_tree_by_hash(&item.id.to_string()).await?;
                stack.push((p, child));
            } else {
                result.push((p, item.id));
            }
        }
    }
    Ok(result)
}

/// List commit history using time-priority traversal (all reachable commits),
/// with optional path and author filters and pagination.
pub async fn list_commit_history<T: ApiHandler + ?Sized>(
    handler: &T,
    start_refs: Option<&str>,
    path_filter: Option<&PathBuf>,
    author: Option<&str>,
    page: Pagination,
) -> Result<(Vec<CommitSummary>, u64), GitError> {
    // Normalize author: empty/whitespace treated as None
    let author_norm = author.map(|s| s.trim()).filter(|t| !t.is_empty());
    // Two-tier cache strategy:
    // 1) Cache the FULL filtered commit index (list of SHAs) keyed by path/refs/author
    // 2) Apply pagination in-memory for any page requests
    let cache_key_index = format!(
        "{}:history_index:v1:path={}:refs={}:author={}",
        handler.object_cache().prefix,
        path_filter
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string()),
        start_refs.unwrap_or_default(),
        author_norm.unwrap_or("__none__"),
    );

    let mut conn = handler.object_cache().connection.clone();
    if let Ok(Some(json)) = conn.get::<_, Option<String>>(&cache_key_index).await
        && let Ok(index) = serde_json::from_str::<Vec<String>>(&json)
    {
        // Renew TTL on cache hit to keep hot indexes alive
        if let Err(e) = conn.expire::<_, ()>(&cache_key_index, 300).await {
            tracing::warn!("failed to renew ttl for {}: {}", &cache_key_index, e);
        }
        let total = index.len() as u64;
        let start = page.page.saturating_sub(1) * page.per_page;
        let end = (start + page.per_page).min(total);

        // Build summaries for the requested page from cached SHAs
        let mut res = Vec::with_capacity((end - start) as usize);
        for sha in &index[start as usize..end as usize] {
            let c = handler.get_commit_by_hash(sha).await?;
            let mut wrapper: LatestCommitInfoWrapper = c.clone().into();
            apply_username_binding(handler, &c.id.to_string(), &mut wrapper).await;
            let info = wrapper.0;
            let gpg_status = compute_gpg_status_for_commit(handler, &c, &info.committer).await;
            res.push(CommitSummary {
                sha: c.id.to_string(),
                short_message: info.short_message,
                author: info.author,
                committer: info.committer,
                date: info.date,
                parents: c.parent_commit_ids.iter().map(|p| p.to_string()).collect(),
                gpg_status,
            });
        }
        return Ok((res, total));
    }

    // Use history traversal with time-priority order to collect commits
    const MAX_SCAN: usize = 10_000;
    let traversed =
        traverse_history_commits(handler, start_refs, path_filter, author_norm, MAX_SCAN).await?;

    // Build and cache the index of SHAs for this filtered history
    let index: Vec<String> = traversed.iter().map(|c| c.id.to_string()).collect();
    match serde_json::to_string(&index) {
        Ok(json) => {
            if let Err(e) = conn.set_ex::<_, _, ()>(&cache_key_index, json, 300).await {
                tracing::warn!("failed to set cache {}: {}", &cache_key_index, e);
            }
        }
        Err(e) => tracing::warn!(
            "failed to serialize history index for {}: {}",
            &cache_key_index,
            e
        ),
    }

    // Paginate locally from traversed commits
    let total = traversed.len() as u64;
    let start = page.page.saturating_sub(1) * page.per_page;
    let end = (start + page.per_page).min(traversed.len() as u64);
    let slice = &traversed[start as usize..end as usize];

    let mut res = Vec::with_capacity(slice.len());
    for c in slice {
        let mut wrapper: LatestCommitInfoWrapper = c.clone().into();
        apply_username_binding(handler, &c.id.to_string(), &mut wrapper).await;
        let info = wrapper.0;
        let gpg_status = compute_gpg_status_for_commit(handler, c, &info.committer).await;
        res.push(CommitSummary {
            sha: c.id.to_string(),
            short_message: info.short_message,
            author: info.author,
            committer: info.committer,
            date: info.date,
            parents: c.parent_commit_ids.iter().map(|p| p.to_string()).collect(),
            gpg_status,
        });
    }

    Ok((res, total))
}

fn build_mui_tree_from_paths(paths: &[String]) -> Vec<MuiTreeNode> {
    let mut roots: Vec<MuiTreeNode> = Vec::new();

    for path in paths {
        let parts: Vec<&str> = path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        if parts.is_empty() {
            continue;
        }

        let root_label = parts[0];
        if let Some(existing_root) = roots.iter_mut().find(|node| node.label == root_label) {
            let mut buf = existing_root.path.clone();
            existing_root.insert_path(&parts[1..], &mut buf);
        } else {
            let mut buf = String::new();
            buf.push('/');
            buf.push_str(root_label);
            let mut new_root = MuiTreeNode::new(root_label, &buf);
            new_root.insert_path(&parts[1..], &mut buf);
            roots.push(new_root);
        }
    }

    roots
}

fn pathbuf_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

async fn compute_changed_paths<T: ApiHandler + ?Sized>(
    handler: &T,
    commit: &Commit,
) -> Result<Vec<String>, GitError> {
    let new_blobs = collect_commit_blobs(handler, commit).await?;
    let new_map: HashMap<String, ObjectHash> = new_blobs
        .into_iter()
        .map(|(path, hash)| (pathbuf_to_string(&path), hash))
        .collect();

    let mut parent_maps: Vec<HashMap<String, ObjectHash>> = Vec::new();
    for &pid in &commit.parent_commit_ids {
        let parent = handler.get_commit_by_hash(&pid.to_string()).await?;
        let blobs = collect_commit_blobs(handler, &parent).await?;
        let map = blobs
            .into_iter()
            .map(|(path, hash)| (pathbuf_to_string(&path), hash))
            .collect::<HashMap<_, _>>();
        parent_maps.push(map);
    }

    let mut candidates: BTreeSet<String> = new_map.keys().cloned().collect();
    for parent_map in &parent_maps {
        candidates.extend(parent_map.keys().cloned());
    }

    if parent_maps.is_empty() {
        return Ok(candidates.into_iter().collect());
    }

    let mut changed = Vec::new();
    'candidate: for path in candidates {
        let new_hash = new_map.get(&path).copied();
        for parent_map in &parent_maps {
            let parent_hash = parent_map.get(&path).copied();
            if parent_hash == new_hash {
                continue 'candidate;
            }
        }
        changed.push(path);
    }

    Ok(changed)
}

async fn assemble_commit_summary<T: ApiHandler + ?Sized>(
    handler: &T,
    commit: &Commit,
) -> CommitSummary {
    let mut wrapper: LatestCommitInfoWrapper = commit.clone().into();
    apply_username_binding(handler, &commit.id.to_string(), &mut wrapper).await;
    let info = wrapper.0;
    let gpg_status = compute_gpg_status_for_commit(handler, commit, &info.committer).await;
    CommitSummary {
        sha: commit.id.to_string(),
        short_message: info.short_message,
        author: info.author,
        committer: info.committer,
        date: info.date,
        parents: commit
            .parent_commit_ids
            .iter()
            .map(|p| p.to_string())
            .collect(),
        gpg_status,
    }
}

async fn compute_commit_diff_items<T: ApiHandler + ?Sized>(
    handler: &T,
    commit: &Commit,
    filter_paths: Option<&[String]>,
) -> Result<Vec<DiffItem>, GitError> {
    let new_blobs = collect_commit_blobs(handler, commit).await?;
    let mut all_hashes: HashSet<ObjectHash> = new_blobs.iter().map(|(_, hash)| *hash).collect();
    let mut parent_blobs_set: Vec<Vec<(PathBuf, ObjectHash)>> = Vec::new();

    for &pid in &commit.parent_commit_ids {
        let parent = handler.get_commit_by_hash(&pid.to_string()).await?;
        let blobs = collect_commit_blobs(handler, &parent).await?;
        for (_, hash) in &blobs {
            all_hashes.insert(*hash);
        }
        parent_blobs_set.push(blobs);
    }

    let mut blob_cache: HashMap<ObjectHash, Vec<u8>> = HashMap::new();
    for hash in &all_hashes {
        // Reuse ApiHandler's blob reading helper so that:
        // - object IDs are validated
        // - ObjStorageNotFound cases are classified/logged via Storage helpers
        match handler.get_raw_blob_by_hash(&hash.to_string()).await {
            Ok(blob) => {
                blob_cache.insert(*hash, blob);
            }
            Err(e) => tracing::warn!("Failed to read blob {}: {}", hash, e),
        }
    }

    let read_content = |file: &PathBuf, hash: &ObjectHash| -> Vec<u8> {
        blob_cache.get(hash).cloned().unwrap_or_else(|| {
            tracing::warn!("Missing blob for {:?} {}", file, hash);
            Vec::new()
        })
    };

    let filters: Vec<PathBuf> = match filter_paths {
        Some(paths) if !paths.is_empty() => paths.iter().map(PathBuf::from).collect(),
        _ => Vec::new(),
    };

    let diff_results: Vec<git_internal::diff::DiffItem> = if parent_blobs_set.is_empty() {
        let empty: Vec<(PathBuf, ObjectHash)> = Vec::new();
        git_internal::diff::Diff::diff(empty, new_blobs, filters, read_content)
    } else {
        let mut combined: HashMap<String, git_internal::diff::DiffItem> = HashMap::new();
        for parent_blobs in parent_blobs_set {
            let diff_items = git_internal::diff::Diff::diff(
                parent_blobs,
                new_blobs.clone(),
                filters.clone(),
                read_content,
            );
            for item in diff_items {
                combined.entry(item.path.clone()).or_insert(item);
            }
        }
        let mut merged: Vec<git_internal::diff::DiffItem> = combined.into_values().collect();
        merged.sort_by(|a, b| a.path.cmp(&b.path));
        merged
    };

    Ok(diff_results.into_iter().collect())
}

#[derive(Serialize, Deserialize)]
struct CachedDiff {
    filter_paths: Option<Vec<String>>,
    items: Vec<DiffItemSchema>,
}

async fn load_or_compute_diff_items<T: ApiHandler + ?Sized>(
    handler: &T,
    commit_opt: Option<&Commit>,
    commit_sha: &str,
    cache_key: &str,
    filter_paths: Option<&[String]>,
) -> Result<Vec<DiffItemSchema>, GitError> {
    let requested_filter = filter_paths.map(|paths| {
        let mut v: Vec<String> = paths.iter().map(|p| p.to_string()).collect();
        v.sort();
        v
    });

    let mut conn = handler.object_cache().connection.clone();
    if let Ok(Some(json)) = conn.get::<_, Option<String>>(cache_key).await {
        match serde_json::from_str::<CachedDiff>(&json) {
            Ok(cached) => {
                if cached.filter_paths == requested_filter {
                    if let Err(e) = conn.expire::<_, ()>(cache_key, 600).await {
                        tracing::warn!("failed to renew ttl for {}: {}", cache_key, e);
                    }
                    return Ok(cached.items);
                }
            }
            Err(e) => tracing::warn!("failed to deserialize diff cache {}: {}", cache_key, e),
        }
    }

    let diffs = if let Some(commit) = commit_opt {
        compute_commit_diff_items(handler, commit, filter_paths).await?
    } else {
        let commit = handler.get_commit_by_hash(commit_sha).await?;
        compute_commit_diff_items(handler, &commit, filter_paths).await?
    };

    let diffs: Vec<DiffItemSchema> = diffs.clone().into_iter().map(|x| x.into()).collect();

    let cache_value = CachedDiff {
        filter_paths: requested_filter,
        items: diffs.clone(),
    };

    match serde_json::to_string(&cache_value) {
        Ok(json) => {
            if let Err(e) = conn.set_ex::<_, _, ()>(cache_key, json, 600).await {
                tracing::warn!("failed to set cache {}: {}", cache_key, e);
            }
        }
        Err(e) => tracing::warn!("failed to serialize diff cache {}: {}", cache_key, e),
    }

    Ok(diffs)
}

/// Build a MUI-compatible tree describing changed files for a commit.
pub async fn get_commit_mui_tree<T: ApiHandler + ?Sized>(
    handler: &T,
    commit_sha: &str,
    selector_path: &std::path::Path,
) -> Result<Vec<MuiTreeNode>, GitError> {
    let selector_str = selector_path.to_string_lossy();
    let tree_cache_key = format!(
        "{}:commit_mui_tree:v1:sha={}:selector={}",
        handler.object_cache().prefix,
        commit_sha,
        selector_str
    );

    let mut conn = handler.object_cache().connection.clone();
    if let Ok(Some(json)) = conn.get::<_, Option<String>>(&tree_cache_key).await
        && let Ok(nodes) = serde_json::from_str::<Vec<MuiTreeNode>>(&json)
    {
        if let Err(e) = conn.expire::<_, ()>(&tree_cache_key, 600).await {
            tracing::warn!("failed to renew ttl for {}: {}", &tree_cache_key, e);
        }
        return Ok(nodes);
    }

    // Cache miss: directly compute changed paths and build MUI tree
    let commit = handler.get_commit_by_hash(commit_sha).await?;
    let all_paths = compute_changed_paths(handler, &commit).await?;
    let nodes = build_mui_tree_from_paths(&all_paths);

    match serde_json::to_string(&nodes) {
        Ok(json) => {
            if let Err(e) = conn.set_ex::<_, _, ()>(&tree_cache_key, json, 600).await {
                tracing::warn!("failed to set cache {}: {}", &tree_cache_key, e);
            }
        }
        Err(e) => tracing::warn!(
            "failed to serialize mui tree cache {}: {}",
            tree_cache_key,
            e
        ),
    }

    Ok(nodes)
}

/// Build paginated diff details for all changed files in a commit, with pagination applied.
pub async fn get_commit_files_changed<T: ApiHandler + ?Sized>(
    handler: &T,
    commit_sha: &str,
    selector_path: &std::path::Path,
    pagination: Pagination,
) -> Result<CommitFilesChangedPage, GitError> {
    let selector_str = selector_path.to_string_lossy();
    let diff_cache_key = format!(
        "{}:commit_diff_index:v1:sha={}:selector={}",
        handler.object_cache().prefix,
        commit_sha,
        selector_str
    );

    let commit = handler.get_commit_by_hash(commit_sha).await?;
    let summary = assemble_commit_summary(handler, &commit).await;

    let diffs =
        load_or_compute_diff_items(handler, Some(&commit), commit_sha, &diff_cache_key, None)
            .await?;

    let total = diffs.len() as u64;
    let per_page = pagination.per_page.max(1);
    let start = pagination.page.saturating_sub(1) * per_page;
    let end = (start + per_page).min(total);

    let items = if start >= end {
        Vec::new()
    } else {
        let skip = start as usize;
        let take = (end - start) as usize;
        diffs.into_iter().skip(skip).take(take).collect()
    };

    Ok(CommitFilesChangedPage {
        commit: summary,
        page: CommonPage { total, items },
    })
}
