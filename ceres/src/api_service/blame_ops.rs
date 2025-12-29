//! Blame operations for tracking line-by-line file history.
//!
//! This module provides a unified blame implementation that works with both
//! MonoApiService and ImportApiService through the ApiHandler trait.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use git_internal::diff::{DiffOperation, compute_diff};
use git_internal::errors::GitError;
use git_internal::hash::SHA1;
use git_internal::internal::object::commit::Commit;
use git_internal::internal::object::tree::TreeItemMode;

use crate::api_service::ApiHandler;
use crate::model::blame::{BlameBlock, BlameInfo, BlameQuery, BlameResult, Contributor};

/// Internal structure to track line attribution during history traversal
struct LineAttribution {
    /// Line number in the final file (1-indexed)
    final_line_number: usize,
    /// The commit that introduced this line
    commit_id: SHA1,
    /// Whether this line's attribution has been finalized
    determined: bool,
}

/// Context for caching data during a single blame operation.
struct BlameContext {
    /// Cache for blob hash: commit_id -> blob_hash (None if file doesn't exist)
    blob_hash_cache: HashMap<SHA1, Option<SHA1>>,
}

impl BlameContext {
    fn new() -> Self {
        Self {
            blob_hash_cache: HashMap::new(),
        }
    }
}

/// Get file lines at a specific commit.
/// Returns None if the file does not exist at the given commit.
async fn get_file_lines<T: ApiHandler + ?Sized>(
    handler: &T,
    file_path: &Path,
    commit: &Commit,
) -> Result<Option<Arc<Vec<String>>>, GitError> {
    match get_file_content_at_commit(handler, file_path, commit).await {
        Ok(content) => {
            let lines = Arc::new(content.lines().map(|s| s.to_string()).collect());
            Ok(Some(lines))
        }
        Err(e) => {
            tracing::debug!("File not found in commit {}: {}", commit.id, e);
            Ok(None)
        }
    }
}

/// Get blob hash with caching support.
async fn get_blob_hash_cached<T: ApiHandler + ?Sized>(
    handler: &T,
    file_path: &Path,
    commit: &Commit,
    ctx: &mut BlameContext,
) -> Option<SHA1> {
    // Check cache first
    if let Some(cached) = ctx.blob_hash_cache.get(&commit.id) {
        return *cached;
    }

    // Cache miss - fetch from tree
    let hash = get_file_hash_simple(handler, file_path, commit).await;
    ctx.blob_hash_cache.insert(commit.id, hash);
    hash
}

/// Get file content and blob hash together (for efficiency).
async fn get_file_content_and_hash<T: ApiHandler + ?Sized>(
    handler: &T,
    file_path: &Path,
    commit: &Commit,
) -> Result<(String, SHA1), GitError> {
    let root_tree = handler
        .object_cache()
        .get_tree(commit.tree_id, |id| async move {
            handler
                .get_tree_by_hash(&id.to_string())
                .await
                .map_err(|e| common::errors::MegaError::Other(e.to_string()))
        })
        .await
        .map_err(|e| GitError::CustomError(e.to_string()))?;

    let relative_path = handler
        .strip_relative(file_path)
        .map_err(|e| GitError::CustomError(e.to_string()))?;

    let blob_hash = navigate_to_blob(handler, root_tree, &relative_path)
        .await?
        .ok_or_else(|| GitError::CustomError("[code:404] File not found".to_string()))?;

    let blob = handler
        .get_raw_blob_by_hash(&blob_hash.to_string())
        .await
        .map_err(|e| GitError::CustomError(e.to_string()))?;

    let content = String::from_utf8(blob).map_err(|e| GitError::ConversionError(e.to_string()))?;

    Ok((content, blob_hash))
}

/// Collect contributors directly from blame blocks.
fn collect_contributors(blocks: &[BlameBlock]) -> Vec<Contributor> {
    let mut contributor_map: HashMap<String, Contributor> = HashMap::new();

    for block in blocks {
        let email = &block.blame_info.author_email;
        contributor_map
            .entry(email.clone())
            .and_modify(|c| {
                c.total_lines += block.line_count;
                if block.blame_info.commit_time > c.last_commit_time {
                    c.last_commit_time = block.blame_info.commit_time;
                }
                if block.blame_info.author_username.is_some() {
                    c.username = block.blame_info.author_username.clone();
                }
            })
            .or_insert(Contributor {
                email: email.clone(),
                username: block.blame_info.author_username.clone(),
                last_commit_time: block.blame_info.commit_time,
                total_lines: block.line_count,
            });
    }

    let mut contributors: Vec<Contributor> = contributor_map.into_values().collect();
    contributors.sort_by(|a, b| {
        b.total_lines
            .cmp(&a.total_lines)
            .then_with(|| b.last_commit_time.cmp(&a.last_commit_time))
    });
    contributors
}

/// Get blame information for a file.
///
/// This function traces the history of each line in the file to find
/// which commit introduced it. It implements several optimizations:
/// - Redis cache for Commit and Tree objects
/// - In-memory cache for parsed file lines
/// - Early termination when target lines are determined
/// - Large file detection and logging
pub async fn get_file_blame<T: ApiHandler + ?Sized>(
    handler: &T,
    file_path: &str,
    ref_name: Option<&str>,
    query: BlameQuery,
) -> Result<BlameResult, GitError> {
    // Validate input
    if file_path.is_empty() {
        return Err(GitError::CustomError(
            "[code:400] File path cannot be empty".to_string(),
        ));
    }

    let file_path_buf = PathBuf::from(file_path);

    // Create cache context for this blame operation
    let mut ctx = BlameContext::new();

    // Get blame configuration
    let config = handler.get_blame_config();

    // Resolve starting commit from refs
    let start_commit =
        crate::api_service::commit_ops::resolve_start_commit(handler, ref_name).await?;

    // Get file content and blob hash at start commit
    let (current_content, start_blob_hash) =
        get_file_content_and_hash(handler, &file_path_buf, &start_commit).await?;
    let current_lines: Vec<String> = current_content.lines().map(|s| s.to_string()).collect();
    let total_lines = current_lines.len();

    if total_lines == 0 {
        return Ok(BlameResult {
            file_path: file_path.to_string(),
            blocks: vec![],
            total_lines: 0,
            page: query.page,
            page_size: query.page_size,
            earliest_commit_time: 0,
            latest_commit_time: 0,
            contributors: vec![],
        });
    }

    // Large file detection: lines > threshold or size > max_size
    let content_size = current_content.len();
    let max_size = config.get_max_size_bytes().unwrap_or(1024 * 1024); // Default 1MB
    let is_large_file = total_lines > config.max_lines_threshold || content_size > max_size;

    // For large files, enforce pagination
    let mut query = query;
    if is_large_file {
        const MAX_LINES_PER_PAGE: usize = 500;

        // If no pagination specified, require it
        let has_pagination =
            query.start_line.is_some() || query.end_line.is_some() || query.page_size.is_some();

        if !has_pagination {
            return Err(GitError::CustomError(format!(
                "[code:400] File is too large ({} lines, {} bytes). Please use pagination with page_size <= {}",
                total_lines, content_size, MAX_LINES_PER_PAGE
            )));
        }

        // Limit page_size for large files
        match query.page_size {
            Some(size) if size > MAX_LINES_PER_PAGE => {
                query.page_size = Some(MAX_LINES_PER_PAGE);
            }
            None => {
                query.page_size = Some(MAX_LINES_PER_PAGE);
            }
            _ => {}
        }
    }

    // Determine target range for pagination
    let target_range = match (query.start_line, query.end_line) {
        (Some(start), Some(end)) => Some((start, end.min(total_lines))),
        (Some(start), None) => Some((start, total_lines)),
        (None, Some(end)) => Some((1, end.min(total_lines))),
        (None, None) => None, // Process all lines (only for non-large files)
    };

    // Cache start commit's blob hash
    ctx.blob_hash_cache
        .insert(start_commit.id, Some(start_blob_hash));

    // Build line attributions
    let attributions = build_line_attributions(
        handler,
        &file_path_buf,
        &start_commit,
        start_blob_hash,
        &current_lines,
        target_range,
        &mut ctx,
    )
    .await?;

    // Create blame blocks from attributions
    let all_blocks = create_blame_blocks(handler, attributions, &current_lines).await?;

    // Calculate statistics
    let (earliest_commit_time, latest_commit_time) = calculate_time_range(&all_blocks);
    let contributors = collect_contributors(&all_blocks);

    // Apply pagination
    let blocks = apply_pagination(all_blocks, &query);

    Ok(BlameResult {
        file_path: file_path.to_string(),
        blocks,
        total_lines,
        page: query.page,
        page_size: query.page_size,
        earliest_commit_time,
        latest_commit_time,
        contributors,
    })
}

/// Get file content at a specific commit
async fn get_file_content_at_commit<T: ApiHandler + ?Sized>(
    handler: &T,
    file_path: &Path,
    commit: &Commit,
) -> Result<String, GitError> {
    let root_tree = handler
        .object_cache()
        .get_tree(commit.tree_id, |id| async move {
            handler
                .get_tree_by_hash(&id.to_string())
                .await
                .map_err(|e| common::errors::MegaError::Other(e.to_string()))
        })
        .await
        .map_err(|e| GitError::CustomError(format!("Failed to get root tree: {}", e)))?;

    let relative_path = handler
        .strip_relative(file_path)
        .map_err(|e| GitError::CustomError(format!("Failed to process path: {}", e)))?;

    let blob_hash = navigate_to_blob(handler, root_tree, &relative_path)
        .await?
        .ok_or_else(|| {
            GitError::CustomError(format!(
                "[code:404] File not found: {}",
                file_path.display()
            ))
        })?;

    // Get blob content
    let blob = handler
        .get_raw_blob_by_hash(&blob_hash.to_string())
        .await
        .map_err(|e| GitError::CustomError(format!("Failed to get blob: {}", e)))?;

    String::from_utf8(blob)
        .map_err(|e| GitError::ConversionError(format!("Invalid UTF-8 in blob: {}", e)))
}

/// Navigate through tree structure to find a blob.
async fn navigate_to_blob<T: ApiHandler + ?Sized>(
    handler: &T,
    root_tree: Arc<git_internal::internal::object::tree::Tree>,
    path: &Path,
) -> Result<Option<SHA1>, GitError> {
    // Skip RootDir component for consistent path handling
    let components: Vec<&str> = path
        .components()
        .filter(|c| !matches!(c, Component::RootDir))
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    if components.is_empty() {
        return Ok(None);
    }

    let mut current_tree = root_tree;

    for (i, component) in components.iter().enumerate() {
        let is_last = i == components.len() - 1;

        let item = current_tree
            .tree_items
            .iter()
            .find(|item| item.name == *component);

        match item {
            Some(item) if is_last => {
                if item.mode == TreeItemMode::Blob || item.mode == TreeItemMode::BlobExecutable {
                    return Ok(Some(item.id));
                }
                return Ok(None);
            }
            Some(item) if item.mode == TreeItemMode::Tree => {
                current_tree = handler
                    .object_cache()
                    .get_tree(item.id, |tree_id| async move {
                        handler
                            .get_tree_by_hash(&tree_id.to_string())
                            .await
                            .map_err(|e| common::errors::MegaError::Other(e.to_string()))
                    })
                    .await
                    .map_err(|e| GitError::CustomError(format!("Failed to get subtree: {}", e)))?;
            }
            _ => return Ok(None),
        }
    }

    Ok(None)
}

/// Build line attributions by tracing through commit history.
///
/// This function implements several optimizations:
/// - Redis cache for Commit objects via object_cache()
/// - In-memory cache for parsed file lines via BlameContext
/// - Early termination when all target lines are determined
/// - Optional target_range for partial file blame
/// - Merge commit traversal: checks ALL parents to find the true source of each line
/// - Blob hash comparison for TREESAME fast path (skip diff if identical)
async fn build_line_attributions<T: ApiHandler + ?Sized>(
    handler: &T,
    file_path: &Path,
    start_commit: &Commit,
    start_blob_hash: SHA1,
    current_lines: &[String],
    target_range: Option<(usize, usize)>,
    ctx: &mut BlameContext,
) -> Result<Vec<LineAttribution>, GitError> {
    let total_lines = current_lines.len();
    if total_lines == 0 {
        return Ok(vec![]);
    }

    // Determine the range of lines to track
    let (track_start, track_end) = match target_range {
        Some((start, end)) => (start.max(1), end.min(total_lines)),
        None => (1, total_lines),
    };

    if track_start > track_end {
        return Ok(vec![]);
    }

    // Initialize attributions for target range only
    let mut attributions: Vec<LineAttribution> = (track_start..=track_end)
        .map(|i| LineAttribution {
            final_line_number: i,
            commit_id: start_commit.id,
            determined: false,
        })
        .collect();

    // Track how many lines still need attribution
    let mut pending_count = attributions.len();

    // Create Arc for initial file content, shared across all line states
    let initial_content = Arc::new(current_lines.to_vec());

    // Track each line's traversal state independently
    // This allows different lines to follow different parent branches in merge commits
    let mut line_states: Vec<LineTraversalState> = attributions
        .iter()
        .enumerate()
        .map(|(idx, attr)| LineTraversalState {
            attr_index: idx,
            current_commit: start_commit.clone(),
            current_line_number: attr.final_line_number,
            content_lines: Arc::clone(&initial_content),
            blob_hash: Some(start_blob_hash),
        })
        .collect();

    // Iteration limit to prevent infinite loops
    const MAX_ITERATIONS: usize = 10_000;
    let mut iteration = 0;

    // Process lines until all are determined
    while pending_count > 0 {
        iteration += 1;
        if iteration > MAX_ITERATIONS {
            tracing::warn!(
                "Blame exceeded max iterations ({}) for file: {}",
                MAX_ITERATIONS,
                file_path.display()
            );
            break;
        }

        // Group undetermined lines by their current commit for batch processing
        let mut commit_groups: HashMap<SHA1, Vec<usize>> = HashMap::new();
        for (idx, state) in line_states.iter().enumerate() {
            if !attributions[state.attr_index].determined {
                commit_groups
                    .entry(state.current_commit.id)
                    .or_default()
                    .push(idx);
            }
        }

        if commit_groups.is_empty() {
            break;
        }

        let mut any_progress = false;

        // Process each commit group
        for (_commit_id, line_indices) in commit_groups {
            // Get the commit (all lines in this group share the same commit)
            let state_idx = line_indices[0];
            let current_commit = line_states[state_idx].current_commit.clone();
            let current_content_lines = line_states[state_idx].content_lines.clone();

            if current_commit.parent_commit_ids.is_empty() {
                // No parents - these lines were introduced in this commit (root commit)
                for &idx in &line_indices {
                    let attr_idx = line_states[idx].attr_index;
                    if !attributions[attr_idx].determined {
                        attributions[attr_idx].determined = true;
                        pending_count -= 1;
                        any_progress = true;
                    }
                }
                continue;
            }

            // Fetch all parents with blob hash fast path
            let mut parent_data: Vec<ParentBlameData> = Vec::new();
            let current_blob_hash = line_states[state_idx].blob_hash;

            for &parent_id in &current_commit.parent_commit_ids {
                let parent_commit = match handler
                    .object_cache()
                    .get_commit(parent_id, |id| async move {
                        handler
                            .get_commit_by_hash(&id.to_string())
                            .await
                            .map_err(|e| common::errors::MegaError::Other(e.to_string()))
                    })
                    .await
                {
                    Ok(c) => (*c).clone(),
                    Err(e) => {
                        tracing::debug!("Failed to get parent commit {}: {}", parent_id, e);
                        continue;
                    }
                };

                // If root tree is identical, blob hash must be identical
                let parent_blob_hash = if parent_commit.tree_id == current_commit.tree_id {
                    current_blob_hash
                } else {
                    get_blob_hash_cached(handler, file_path, &parent_commit, ctx).await
                };

                // TREESAME: if blob hash is identical, skip diff
                if let (Some(curr_hash), Some(parent_hash)) = (current_blob_hash, parent_blob_hash)
                    && curr_hash == parent_hash
                {
                    // File identical, no diff needed
                    parent_data.clear();
                    parent_data.push(ParentBlameData {
                        commit: parent_commit,
                        blob_hash: Some(parent_hash),
                        file_lines: None, // Don't need content
                        line_map: HashMap::new(),
                        is_identical: true,
                    });
                    break; // Only use this TREESAME parent, skip other parents
                }

                // File changed - get content and compute diff
                let parent_lines = get_file_lines(handler, file_path, &parent_commit).await?;

                // Compute line mapping if file exists in parent
                let line_map = if let Some(ref p_lines) = parent_lines {
                    let diff_ops = compute_diff(p_lines, &current_content_lines);
                    let mut map: HashMap<usize, usize> = HashMap::new();
                    for op in &diff_ops {
                        if let DiffOperation::Equal { old_line, new_line } = op {
                            map.insert(*new_line, *old_line);
                        }
                    }
                    map
                } else {
                    HashMap::new()
                };

                parent_data.push(ParentBlameData {
                    commit: parent_commit,
                    blob_hash: parent_blob_hash,
                    file_lines: parent_lines,
                    line_map,
                    is_identical: false,
                });
            }

            // For each line in this group, find which parent (if any) has it
            for &idx in &line_indices {
                let attr_idx = line_states[idx].attr_index;
                if attributions[attr_idx].determined {
                    continue;
                }

                let current_line_num = line_states[idx].current_line_number;
                let mut found_in_parent = false;

                // Check each parent to find where this line came from
                for parent in &parent_data {
                    if parent.is_identical {
                        // Fast path: file identical, 1:1 line mapping
                        attributions[attr_idx].commit_id = parent.commit.id;
                        line_states[idx].current_commit = parent.commit.clone();
                        line_states[idx].blob_hash = parent.blob_hash;
                        // Line number and content unchanged
                        found_in_parent = true;
                        any_progress = true;
                        break;
                    } else if let Some(ref parent_lines) = parent.file_lines
                        && let Some(&parent_line_num) = parent.line_map.get(&current_line_num)
                    {
                        // Line exists in this parent - continue tracing
                        attributions[attr_idx].commit_id = parent.commit.id;
                        line_states[idx].current_commit = parent.commit.clone();
                        line_states[idx].current_line_number = parent_line_num;
                        line_states[idx].content_lines = Arc::clone(parent_lines);
                        line_states[idx].blob_hash = parent.blob_hash;
                        found_in_parent = true;
                        any_progress = true;
                        break;
                    }
                }

                if !found_in_parent {
                    // Line doesn't exist in any parent - it was introduced in current commit
                    attributions[attr_idx].determined = true;
                    pending_count -= 1;
                    any_progress = true;
                }
            }
        }

        // If no progress was made, we're stuck (shouldn't happen normally)
        if !any_progress {
            tracing::warn!(
                "Blame made no progress at iteration {} for file: {}",
                iteration,
                file_path.display()
            );
            break;
        }
    }

    Ok(attributions)
}

/// State for tracking a single line's traversal through commit history
struct LineTraversalState {
    /// Index into the attributions vector
    attr_index: usize,
    /// Current commit being examined for this line
    current_commit: Commit,
    /// Line number in the current commit's version of the file
    current_line_number: usize,
    /// File content at current commit (shared via Arc)
    content_lines: Arc<Vec<String>>,
    /// Blob hash of the file at current commit (for TREESAME check)
    blob_hash: Option<SHA1>,
}

/// Data about a parent commit for blame traversal
struct ParentBlameData {
    /// The parent commit
    commit: Commit,
    /// Blob hash of the file at this parent
    blob_hash: Option<SHA1>,
    /// File content lines at this parent (None if file doesn't exist or identical)
    file_lines: Option<Arc<Vec<String>>>,
    /// Line number mapping: current_line -> parent_line (for unchanged lines)
    line_map: HashMap<usize, usize>,
    /// True if file is identical to current (TREESAME - skip diff)
    is_identical: bool,
}

/// Create blame blocks by grouping consecutive lines with same commit
async fn create_blame_blocks<T: ApiHandler + ?Sized>(
    handler: &T,
    attributions: Vec<LineAttribution>,
    lines: &[String],
) -> Result<Vec<BlameBlock>, GitError> {
    if attributions.is_empty() {
        return Ok(vec![]);
    }

    let mut blocks = Vec::new();
    let mut block_start = 0;
    let mut current_commit_id = attributions[0].commit_id;

    for (i, attr) in attributions.iter().enumerate() {
        if attr.commit_id != current_commit_id {
            // Create block for previous group
            let block = create_single_block(handler, &attributions[block_start..i], lines).await?;
            blocks.push(block);

            block_start = i;
            current_commit_id = attr.commit_id;
        }
    }

    // Create final block
    let block = create_single_block(handler, &attributions[block_start..], lines).await?;
    blocks.push(block);

    Ok(blocks)
}

/// Create a single blame block from consecutive attributions with the same commit
async fn create_single_block<T: ApiHandler + ?Sized>(
    handler: &T,
    attrs: &[LineAttribution],
    lines: &[String],
) -> Result<BlameBlock, GitError> {
    if attrs.is_empty() {
        return Err(GitError::CustomError("Empty attribution slice".to_string()));
    }

    let first_attr = &attrs[0];
    let commit_id = first_attr.commit_id;

    // Get commit info using Redis cache
    let commit = (*handler
        .object_cache()
        .get_commit(commit_id, |id| async move {
            handler
                .get_commit_by_hash(&id.to_string())
                .await
                .map_err(|e| common::errors::MegaError::Other(e.to_string()))
        })
        .await
        .map_err(|e| GitError::CustomError(format!("Failed to get commit: {}", e)))?)
    .clone();

    let commit_hash_str = commit_id.to_string();

    // Try to get username binding
    let author_username = if let Ok(Some(binding)) =
        handler.build_commit_binding_info(&commit_hash_str).await
        && !binding.is_anonymous
        && let Some(username) = binding.matched_username
    {
        Some(username)
    } else {
        // Fallback: extract from email
        Some(extract_username_from_email(&commit.author.email))
    };

    // Clean GPG signature from commit message (applied at presentation layer)
    let clean_message = clean_commit_message(&commit.message);
    let commit_summary = clean_message.lines().next().unwrap_or("").to_string();

    // Use final_line_number as the original line number since we track from there
    let blame_info = BlameInfo {
        commit_hash: commit_hash_str.clone(),
        commit_short_id: commit_hash_str.chars().take(7).collect(),
        author_username,
        author_email: commit.author.email.clone(),
        commit_time: commit.committer.timestamp as i64,
        commit_message: clean_message,
        commit_summary,
        original_line_number: first_attr.final_line_number,
        commit_detail_url: format!("/commit/{}", commit_hash_str),
    };

    // Collect content using final_line_number (the line position in the final file)
    let start_line = attrs.first().map(|a| a.final_line_number).unwrap_or(1);
    let end_line = attrs.last().map(|a| a.final_line_number).unwrap_or(1);
    let content: Vec<&str> = attrs
        .iter()
        .filter_map(|a| lines.get(a.final_line_number - 1).map(|s| s.as_str()))
        .collect();

    Ok(BlameBlock {
        content: content.join("\n"),
        blame_info,
        start_line,
        end_line,
        line_count: attrs.len(),
    })
}

/// Apply pagination to blame blocks
fn apply_pagination(blocks: Vec<BlameBlock>, query: &BlameQuery) -> Vec<BlameBlock> {
    if blocks.is_empty() {
        return vec![];
    }

    let (target_start, target_end) = match (query.page, query.page_size) {
        (Some(page), Some(page_size)) if page > 0 && page_size > 0 => {
            let start = (page - 1) * page_size + 1;
            let end = start + page_size - 1;
            (start, end)
        }
        _ => match (query.start_line, query.end_line) {
            (Some(start), Some(end)) => (start, end),
            (Some(start), None) => (start, usize::MAX),
            (None, Some(end)) => (1, end),
            (None, None) => return blocks,
        },
    };

    blocks
        .into_iter()
        .filter(|b| b.end_line >= target_start && b.start_line <= target_end)
        .collect()
}

/// Calculate earliest and latest commit times
fn calculate_time_range(blocks: &[BlameBlock]) -> (i64, i64) {
    if blocks.is_empty() {
        return (0, 0);
    }

    let mut earliest = blocks[0].blame_info.commit_time;
    let mut latest = blocks[0].blame_info.commit_time;

    for block in blocks {
        if block.blame_info.commit_time < earliest {
            earliest = block.blame_info.commit_time;
        }
        if block.blame_info.commit_time > latest {
            latest = block.blame_info.commit_time;
        }
    }

    (earliest, latest)
}

/// Simple file hash lookup (no caching)
async fn get_file_hash_simple<T: ApiHandler + ?Sized>(
    handler: &T,
    file_path: &Path,
    commit: &Commit,
) -> Option<SHA1> {
    let root_tree = handler
        .object_cache()
        .get_tree(commit.tree_id, |id| async move {
            handler
                .get_tree_by_hash(&id.to_string())
                .await
                .map_err(|e| common::errors::MegaError::Other(e.to_string()))
        })
        .await
        .ok()?;

    let relative_path = handler.strip_relative(file_path).ok()?;

    navigate_to_blob(handler, root_tree, &relative_path)
        .await
        .ok()
        .flatten()
}

/// Extract username from email address.
///
/// Handles special formats like GitHub noreply emails:
/// - `123456+username@users.noreply.github.com` → `username`
/// - `username@users.noreply.github.com` → `username`
/// - `user@example.com` → `user`
fn extract_username_from_email(email: &str) -> String {
    let local_part = email.split('@').next().unwrap_or(email);

    // Handle GitHub noreply format: "123456+username" or just "username"
    if email.ends_with("@users.noreply.github.com") {
        // Extract username after the "+" if present
        if let Some(pos) = local_part.find('+') {
            return local_part[pos + 1..].to_string();
        }
    }

    local_part.to_string()
}

/// Remove GPG signature header from commit message if present.
///
/// Git commit objects may contain a `gpgsig` header with PGP signature data.
/// This function removes the signature block and returns only the actual
/// commit message content.
fn clean_commit_message(raw_message: &str) -> String {
    if !raw_message.starts_with("gpgsig ") {
        return raw_message.to_string();
    }

    let mut lines = raw_message.lines().peekable();
    let mut in_signature = true;
    let mut message_lines = Vec::new();

    while let Some(line) = lines.next() {
        if in_signature {
            if line.contains("-----END PGP SIGNATURE-----") {
                in_signature = false;
                // Skip empty lines after signature block
                while let Some(&next_line) = lines.peek() {
                    if next_line.trim().is_empty() || next_line.starts_with(' ') {
                        lines.next();
                    } else {
                        break;
                    }
                }
            }
            continue;
        }
        message_lines.push(line);
    }

    message_lines.join("\n")
}
