//! # Blame Service
//!
//! Provides an efficient, line-by-line blame functionality for the Mega system.
//!
//! ## Key Features
//!
//! - **Accurate Attribution**: Traces commit history to find the origin of each line.
//! - **High Performance**: Uses a caching layer to minimize database lookups.
//! - **Large File Handling**: Employs a chunked strategy for memory efficiency.
//! - **Frontend Friendly**: Delivers a comprehensive and easy-to-use data model.

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use crate::model::blame_dto::{
    BlameBlock, BlameCandidate, BlameInfo, BlameLine, BlameQuery, BlameResult, Contributor,
    FileVersion, LargeFileConfig, LineAttribution,
};
use crate::storage::{Storage, mono_storage::MonoStorage, raw_db_storage::RawDbStorage};
use common::config::Config;
use git_internal::errors::GitError;
use git_internal::hash::SHA1;
use neptune::{DiffOperation, compute_diff};

use crate::utils::converter::FromMegaModel;
#[cfg(test)]
use crate::utils::converter::IntoMegaModel;
use git_internal::internal::object::commit::Commit;
use git_internal::internal::object::tree::{Tree, TreeItemMode};
use std::sync::{Arc, Weak};
use tokio::sync::RwLock;

/// Maps blame candidates from current commit to parent commit based on diff operations
/// This function ensures that line number contexts are correctly maintained during history traversal
fn map_blame_to_parent(
    current_candidates: &[BlameCandidate],
    diff_ops: &[DiffOperation],
) -> Vec<BlameCandidate> {
    use std::collections::HashMap;

    let mut parent_candidates = Vec::new();

    // Create a mapping from new_line (child commit) to old_line (parent commit)
    let mut line_map = HashMap::new();
    for op in diff_ops {
        if let DiffOperation::Equal { old_line, new_line } = op {
            line_map.insert(*new_line, *old_line);
        }
    }

    for candidate in current_candidates {
        // If this line exists in the parent commit (Equal operation)
        if let Some(&parent_line_number) = line_map.get(&candidate.line_number) {
            // Create a new candidate with updated line number for parent context
            // but preserve the original final line number for attribution
            parent_candidates.push(BlameCandidate {
                line_number: parent_line_number,
                original_final_line_number: candidate.original_final_line_number,
            });
        }
        // If the line doesn't exist in parent (Insert operation),
        // we don't add it to parent_candidates, meaning blame stops here
    }

    parent_candidates
}

/// Cache for storing intermediate blame results
#[derive(Debug)]
struct BlameCache {
    /// Cache for file versions at specific commits
    file_versions: RwLock<HashMap<String, Arc<FileVersion>>>,
    /// Cache for commit objects - Using Arc to avoid unnecessary clones
    commits: RwLock<HashMap<String, Arc<Commit>>>,
}

impl BlameCache {
    fn new() -> Self {
        Self {
            file_versions: RwLock::new(HashMap::new()),
            commits: RwLock::new(HashMap::new()),
        }
    }
}

/// Blame service for tracking line-by-line file history
#[derive(Clone)]
pub struct BlameService {
    mono_storage: Arc<MonoStorage>,
    raw_db_storage: Arc<RawDbStorage>,
    cache: Arc<BlameCache>,
    config: Weak<Config>,
}

impl BlameService {
    /// Create a new BlameService instance using global configuration
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            mono_storage: Arc::new(storage.mono_storage()),
            raw_db_storage: Arc::new(storage.raw_db_storage()),
            cache: Arc::new(BlameCache::new()),
            config: storage.config.clone(),
        }
    }

    /// Create a mock instance for testing
    pub fn mock() -> Self {
        let storage = Arc::new(Storage::mock());
        Self::new(storage)
    }

    /// Normalize ref input into a canonical form (e.g., 'main' or 'refs/heads/main')
    fn normalize_ref_name(input: &str) -> String {
        let t = input.trim().trim_start_matches('/');
        if t.is_empty() {
            "main".to_string()
        } else {
            t.to_string()
        }
    }

    /// Check if a file is considered large based on configuration
    pub async fn check_if_large_file(
        &self,
        file_path: &str,
        ref_name: Option<&str>,
    ) -> Result<bool, GitError> {
        tracing::debug!(
            "Checking if file is large: {} at ref: {:?}",
            file_path,
            ref_name
        );

        // Resolve the commit to analyze
        let commit = self.resolve_commit(ref_name).await?;
        let file_path_buf = PathBuf::from(file_path);

        // Get file blob hash
        let blob_hash = match self.get_file_blob_hash(&file_path_buf, &commit).await? {
            Some(hash) => hash,
            None => {
                tracing::debug!("File not found: {}", file_path);
                return Ok(false); // File doesn't exist, not large
            }
        };

        // Get blob content to check size and line count
        let content = self.get_blob_content(&blob_hash).await?;
        let content_size = content.len();
        let line_count = content.lines().count();

        // Check against thresholds
        let is_large = if let Some(config) = self.config.upgrade() {
            let max_size = config.blame.get_max_size_bytes().unwrap_or(usize::MAX);
            content_size > max_size || line_count > config.blame.max_lines_threshold
        } else {
            let defaults = LargeFileConfig::default();
            content_size > defaults.max_size_threshold || line_count > defaults.max_lines_threshold
        };

        tracing::debug!(
            "File {} size: {} bytes, lines: {}, is_large: {}",
            file_path,
            content_size,
            line_count,
            is_large
        );

        Ok(is_large)
    }

    /// Get blame information for a file
    pub async fn get_file_blame(
        &self,
        file_path: &str,
        ref_name: Option<&str>,
        query: Option<BlameQuery>,
    ) -> Result<BlameResult, GitError> {
        let file_path = PathBuf::from(file_path);

        // Resolve the commit to analyze
        let commit = self.resolve_commit(ref_name).await?;

        // Get the file version at this commit (using cache for better performance)
        let current_version = self.get_file_version_cached(&file_path, &commit).await?;

        // Build line attributions
        let attributions = self
            .build_line_attributions(&file_path, &commit, &current_version)
            .await?;

        // Create blame lines with commit information
        let blame_lines = self.create_blame_lines(attributions).await?;

        // Convert lines to blocks by grouping consecutive lines with same commit
        let all_blocks = self.convert_lines_to_blocks(blame_lines);

        // Calculate total lines from all blocks before pagination
        let total_lines = all_blocks
            .iter()
            .map(|block| block.line_count)
            .sum::<usize>();

        // Apply pagination to blocks based on line numbers
        let blocks = if let Some(ref q) = query {
            self.apply_pagination_to_blocks(all_blocks, q)
        } else {
            all_blocks
        };

        // Calculate earliest and latest commit times from all blocks
        let (earliest_commit_time, latest_commit_time) = self.calculate_commit_time_range(&blocks);

        // Collect contributors from blocks
        let contributors = self.collect_contributors(&blocks).await?;

        Ok(BlameResult {
            file_path: file_path.to_string_lossy().to_string(),
            blocks,
            total_lines,
            page: query.as_ref().and_then(|q| q.page),
            page_size: query.as_ref().and_then(|q| q.page_size),
            earliest_commit_time,
            latest_commit_time,
            contributors,
        })
    }

    /// Resolve commit from reference name or use HEAD (with caching)
    async fn resolve_commit(&self, ref_name: Option<&str>) -> Result<Commit, GitError> {
        // Step 1: Attempt to fetch from cache
        let cache_key = match ref_name {
            Some(s) => Self::normalize_ref_name(s),
            None => "main".to_string(),
        };

        {
            let cache = self.cache.commits.read().await;
            if let Some(cached_commit) = cache.get(&cache_key) {
                return Ok((**cached_commit).clone());
            }
        }

        // Step 2: Cache miss, resolve from storage
        let commit = match ref_name {
            Some(ref_str) => {
                let requested = Self::normalize_ref_name(ref_str);

                // Try to resolve by commit hash first
                if let Ok(Some(commit)) = self.get_commit_by_hash(&requested).await {
                    Ok(commit)
                } else if !requested.starts_with("refs/") {
                    // Short branch names (e.g., "main"): prefer root default first
                    let expected_heads = format!("refs/heads/{}", requested);
                    match self.mono_storage.get_ref("/").await {
                        Ok(Some(root_ref)) => {
                            if root_ref.ref_name == expected_heads {
                                match self.get_commit_by_hash(&root_ref.ref_commit_hash).await {
                                    Ok(Some(commit)) => Ok(commit),
                                    _ => {
                                        self.commit_by_ref_or_default(&expected_heads, &requested)
                                            .await
                                    }
                                }
                            } else {
                                // Different name at root, resolve via standard refs/heads only
                                self.commit_by_ref_or_default(&expected_heads, &requested)
                                    .await
                            }
                        }
                        _ => {
                            self.commit_by_ref_or_default(&expected_heads, &requested)
                                .await
                        }
                    }
                } else {
                    // Full ref name: first check root default to avoid path ambiguity, then normal resolution
                    match self.mono_storage.get_ref("/").await {
                        Ok(Some(root_ref)) => {
                            if root_ref.ref_name == requested {
                                match self.get_commit_by_hash(&root_ref.ref_commit_hash).await {
                                    Ok(Some(commit)) => Ok(commit),
                                    _ => {
                                        self.commit_by_ref_or_default(&requested, &requested).await
                                    }
                                }
                            } else {
                                self.commit_by_ref_or_default(&requested, &requested).await
                            }
                        }
                        _ => self.commit_by_ref_or_default(&requested, &requested).await,
                    }
                }
            }
            None => {
                // Default to 'main' branch when no ref is provided
                let requested = "main";
                let expected_heads = format!("refs/heads/{}", requested);
                match self.mono_storage.get_ref("/").await {
                    Ok(Some(root_ref)) => {
                        if root_ref.ref_name == expected_heads {
                            match self.get_commit_by_hash(&root_ref.ref_commit_hash).await {
                                Ok(Some(commit)) => Ok(commit),
                                _ => {
                                    self.commit_by_ref_or_default(&expected_heads, requested)
                                        .await
                                }
                            }
                        } else {
                            self.commit_by_ref_or_default(&expected_heads, requested)
                                .await
                        }
                    }
                    _ => {
                        self.commit_by_ref_or_default(&expected_heads, requested)
                            .await
                    }
                }
            }
        };

        // Step 3: Store in cache
        let resolved_commit = commit?;
        {
            let mut cache = self.cache.commits.write().await;
            cache.insert(cache_key.clone(), Arc::new(resolved_commit.clone()));
            tracing::debug!("Cached commit: {}", cache_key);
        }

        Ok(resolved_commit)
    }

    // Extracted helper: resolve commit by ref name with safe fallback
    async fn commit_by_ref_or_default(
        &self,
        full_ref_name: &str,
        ref_display_name: &str,
    ) -> Result<Commit, GitError> {
        // Prefer root default ref when it exactly matches the requested full ref
        if let Ok(Some(default_ref)) = self.mono_storage.get_ref("/").await
            && default_ref.ref_name == full_ref_name
        {
            if let Some(commit) = self
                .get_commit_by_hash(&default_ref.ref_commit_hash)
                .await?
            {
                return Ok(commit);
            } else {
                tracing::warn!(
                    "Default ref {} -> {} missing in mono commits; continuing to standard resolution",
                    full_ref_name,
                    default_ref.ref_commit_hash
                );
            }
        }

        match self.mono_storage.get_ref_by_name(full_ref_name).await {
            Ok(Some(ref_row)) => match self.get_commit_by_hash(&ref_row.ref_commit_hash).await? {
                Some(commit) => Ok(commit),
                None => {
                    tracing::warn!(
                        "Ref {} -> {} missing in mono commits; aborting",
                        full_ref_name,
                        ref_row.ref_commit_hash
                    );
                    Err(GitError::ObjectNotFound(ref_display_name.to_string()))
                }
            },
            Ok(None) => {
                tracing::debug!("Ref not found by name: {}", full_ref_name);
                Err(GitError::ObjectNotFound(ref_display_name.to_string()))
            }
            Err(e) => Err(GitError::CustomError(format!(
                "Failed to resolve reference: {}",
                e
            ))),
        }
    }

    /// Get file version at specific commit (with caching)
    async fn get_file_version(
        &self,
        file_path: &PathBuf,
        commit: &Commit,
    ) -> Result<FileVersion, GitError> {
        // 🚀 Step 1: Attempt to fetch from cache
        let cache_key = format!("{}:{}", commit.id, file_path.display());

        {
            let cache = self.cache.file_versions.read().await;
            if let Some(cached_version) = cache.get(&cache_key) {
                tracing::debug!("Cache hit for file version: {}", cache_key);
                return Ok((**cached_version).clone());
            }
        }

        // 🔄 Step 2: Cache miss, fetch from storage
        tracing::debug!(
            "Cache miss for file version: {}, fetching from storage",
            cache_key
        );
        let blob_hash = self
            .get_file_blob_hash(file_path, commit)
            .await?
            .ok_or_else(|| {
                GitError::ObjectNotFound(format!("File not found: {}", file_path.display()))
            })?;

        let content = self.get_blob_content(&blob_hash).await?;
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        let file_version = FileVersion {
            commit_hash: commit.id,
            blob_hash,
            content,
            lines,
        };

        // 💾 Step 3: Store in cache
        {
            let mut cache = self.cache.file_versions.write().await;
            cache.insert(cache_key.clone(), Arc::new(file_version.clone()));
            tracing::debug!("Cached file version: {}", cache_key);
        }

        Ok(file_version)
    }

    /// Trace line history through commit ancestry using iterative approach to avoid stack overflow
    /// Build line attributions by correctly mapping blame candidates through commit history
    /// This fixes the context mapping error in the original trace_line_history implementation
    async fn build_line_attributions(
        &self,
        file_path: &PathBuf,
        target_commit: &Commit,
        current_version: &FileVersion,
    ) -> Result<Vec<LineAttribution>, GitError> {
        // Initialize final attributions - all lines initially belong to target_commit
        let mut final_attributions: Vec<LineAttribution> = current_version
            .lines
            .iter()
            .enumerate()
            .map(|(i, content)| LineAttribution {
                line_number: i + 1,
                content: content.clone(),
                commit_hash: target_commit.id,
                line_number_in_commit: i + 1,
            })
            .collect();

        // Initialize blame candidates - all lines of the final file need to be traced
        let mut candidates: Vec<BlameCandidate> = (1..=final_attributions.len())
            .map(|i| BlameCandidate {
                line_number: i,
                original_final_line_number: i,
            })
            .collect();

        let mut current_commit = target_commit.clone();

        // History traversal loop with correct context mapping
        while !candidates.is_empty() && !current_commit.parent_commit_ids.is_empty() {
            // Blame algorithm typically follows the first parent commit
            let parent_id = &current_commit.parent_commit_ids[0];
            let parent_commit = match self.get_commit_cached(parent_id).await? {
                Some(c) => c,
                None => {
                    tracing::warn!("Parent commit not found: {}", parent_id);
                    break;
                }
            };

            // Get file versions for current and parent commits
            let child_version = self
                .get_file_version_cached(file_path, &current_commit)
                .await?;
            let parent_version = match self
                .get_file_version_cached(file_path, &parent_commit)
                .await
            {
                Ok(v) => v,
                Err(_) => {
                    tracing::debug!(
                        "File {} not found in parent commit {}, stopping traversal",
                        file_path.display(),
                        parent_commit.id
                    );
                    break;
                }
            };

            // Compute diff between parent and child versions
            let diff_ops = self.compute_blame_diff(&parent_version.lines, &child_version.lines);

            // Map candidates to parent commit context
            let parent_candidates = map_blame_to_parent(&candidates, &diff_ops);

            // Update final attributions for lines that continue to be traced
            for candidate in &parent_candidates {
                let index = candidate.original_final_line_number - 1;
                if let Some(attr) = final_attributions.get_mut(index) {
                    attr.commit_hash = parent_commit.id;
                    attr.line_number_in_commit = candidate.line_number;
                }
            }

            // Prepare for next iteration
            candidates = parent_candidates;
            current_commit = parent_commit;
        }

        Ok(final_attributions)
    }

    /// Compute diff operations between two file versions using Neptune's Myers algorithm
    fn compute_blame_diff(&self, old_lines: &[String], new_lines: &[String]) -> Vec<DiffOperation> {
        // Use Neptune's Myers algorithm which directly returns DiffOperation
        compute_diff(old_lines, new_lines)
    }

    /// Get campsite username from commit_auths table by commit hash and email
    async fn get_campsite_username(&self, commit_hash: &str, email: &str) -> Option<String> {
        let commit_binding_storage = self.mono_storage.commit_binding_storage();

        // Try to find binding by commit hash
        if let Ok(Some(binding)) = commit_binding_storage.find_by_sha(commit_hash).await {
            // Check if the email matches and user is not anonymous
            if binding.author_email == email && !binding.is_anonymous {
                return binding.matched_username;
            }
        }

        None
    }

    /// Get user info (email and username) from commit_auths table
    /// Returns None if no binding found, caller should use git commit info as fallback
    async fn get_campsite_user_info(&self, commit_hash: &str) -> Option<(String, Option<String>)> {
        let commit_binding_storage = self.mono_storage.commit_binding_storage();

        // Try to find binding by commit hash
        if let Ok(Some(binding)) = commit_binding_storage.find_by_sha(commit_hash).await {
            if !binding.is_anonymous {
                return Some((binding.author_email, binding.matched_username));
            } else {
                // If anonymous, return email but no username
                return Some((binding.author_email, None));
            }
        }

        // No binding found
        None
    }

    /// Collect contributors from blame blocks
    async fn collect_contributors(
        &self,
        blocks: &[BlameBlock],
    ) -> Result<Vec<Contributor>, GitError> {
        use std::collections::HashMap;

        let mut contributor_map: HashMap<String, Contributor> = HashMap::new();

        for block in blocks {
            let email = &block.blame_info.author_email;
            let commit_time = block.blame_info.author_time;
            let line_count = block.line_count;

            // Use email as the key to group contributors
            let key = email.clone();

            if let Some(existing_contributor) = contributor_map.get_mut(&key) {
                // Update existing contributor
                existing_contributor.total_lines += line_count;
                if commit_time > existing_contributor.last_commit_time {
                    existing_contributor.last_commit_time = commit_time;
                }
            } else {
                // Create new contributor
                let username = self
                    .get_campsite_username(&block.blame_info.commit_hash, email)
                    .await;

                let contributor = Contributor {
                    email: email.clone(),
                    username,
                    last_commit_time: commit_time,
                    total_lines: line_count,
                };

                contributor_map.insert(key, contributor);
            }
        }

        // Convert to vector and sort by total lines (descending)
        let mut contributors: Vec<Contributor> = contributor_map.into_values().collect();
        contributors.sort_by(|a, b| b.total_lines.cmp(&a.total_lines));

        Ok(contributors)
    }

    /// Calculate earliest and latest commit times from blame blocks
    fn calculate_commit_time_range(&self, blocks: &[BlameBlock]) -> (i64, i64) {
        if blocks.is_empty() {
            return (0, 0);
        }

        // Initialize earliest and latest to the first block's commit time
        let first_commit_time = blocks[0].blame_info.author_time;
        let mut earliest = first_commit_time;
        let mut latest = first_commit_time;

        // Start from the second block since we already used the first one for initialization
        for block in &blocks[1..] {
            let commit_time = block.blame_info.author_time;
            if commit_time < earliest {
                earliest = commit_time;
            }
            if commit_time > latest {
                latest = commit_time;
            }
        }

        (earliest, latest)
    }

    /// Create blame lines with commit information
    async fn create_blame_lines(
        &self,
        attributions: Vec<LineAttribution>,
    ) -> Result<Vec<BlameLine>, GitError> {
        let mut blame_lines = Vec::new();

        for attr in attributions {
            // Get commit info using cached method
            let commit = match self.get_commit_cached(&attr.commit_hash).await {
                Ok(Some(commit)) => commit,
                Ok(None) => {
                    return Err(GitError::ObjectNotFound(format!(
                        "Commit not found: {}",
                        attr.commit_hash
                    )));
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to get commit {}: {}",
                        attr.commit_hash.to_string(),
                        e
                    );
                    return Err(e);
                }
            };

            let author_time = commit.author.timestamp as i64;
            let commit_hash_str = attr.commit_hash.to_string();

            // Get campsite username from commit_auths table
            // Try to get author info from commit_auths table, fallback to git commit
            let (author_email, author_username) = if let Some((email, username)) =
                self.get_campsite_user_info(&commit_hash_str).await
            {
                (email, username)
            } else {
                let git_email = commit.author.email.clone();
                let username = self
                    .get_campsite_username(&commit_hash_str, &git_email)
                    .await;
                (git_email, username)
            };

            // For committer, we still need to get from git commit and try to find username
            let committer_email = commit.committer.email.clone();
            let committer_username = self
                .get_campsite_username(&commit_hash_str, &committer_email)
                .await;

            let blame_info = BlameInfo {
                commit_hash: commit_hash_str.clone(),
                commit_short_id: commit_hash_str.chars().take(7).collect(),
                author_email,
                author_time,
                committer_email,
                committer_time: commit.committer.timestamp as i64,
                commit_message: commit.message.clone(),
                commit_summary: commit.message.lines().next().unwrap_or("").to_string(),
                original_line_number: attr.line_number_in_commit,
                author_username,
                committer_username,
                commit_detail_url: format!("/commit/{}", commit_hash_str),
            };

            blame_lines.push(BlameLine {
                line_number: attr.line_number,
                content: attr.content,
                blame_info,
            });
        }

        Ok(blame_lines)
    }

    /// Apply pagination to blame blocks based on line numbers
    fn apply_pagination_to_blocks(
        &self,
        blocks: Vec<BlameBlock>,
        q: &BlameQuery,
    ) -> Vec<BlameBlock> {
        if blocks.is_empty() {
            return Vec::new();
        }

        // Calculate the line range we want to display
        let (target_start_line, target_end_line) = if q.page.is_some() && q.page_size.is_some() {
            let page = q.page.unwrap();
            let page_size = q.page_size.unwrap();

            if page == 0 || page_size == 0 {
                return Vec::new();
            }

            // If start_line and end_line are specified, apply pagination within that range
            if q.start_line.is_some() || q.end_line.is_some() {
                let range_start = q.start_line.unwrap_or(1);
                let range_end = q.end_line.unwrap_or(usize::MAX);

                // Calculate pagination within the specified range
                let range_size = range_end - range_start + 1;
                let page_start_offset = (page - 1) * page_size;

                if page_start_offset >= range_size {
                    return Vec::new();
                }

                let page_end_offset = (page_start_offset + page_size).min(range_size);
                let target_start = range_start + page_start_offset;
                let target_end = range_start + page_end_offset - 1;

                (target_start, target_end)
            } else {
                // Normal pagination from the beginning
                let target_start = (page - 1) * page_size + 1;
                let target_end = target_start + page_size - 1;
                (target_start, target_end)
            }
        } else if q.start_line.is_some() || q.end_line.is_some() {
            // No pagination, just line range filtering
            let start = q.start_line.unwrap_or(1);
            let end = q.end_line.unwrap_or(usize::MAX);
            (start, end)
        } else {
            // No pagination and no line range, return all blocks
            return blocks;
        };

        // Filter and adjust blocks based on the target line range
        let mut result_blocks = Vec::new();

        for block in blocks {
            // Check if this block intersects with our target range
            if block.end_line < target_start_line || block.start_line > target_end_line {
                continue; // Block is completely outside our range
            }

            // Calculate the intersection of block lines and target range
            let intersection_start = block.start_line.max(target_start_line);
            let intersection_end = block.end_line.min(target_end_line);

            if intersection_start <= intersection_end {
                // Create a new block with only the lines in our target range
                let adjusted_block =
                    self.create_adjusted_block(&block, intersection_start, intersection_end);
                result_blocks.push(adjusted_block);
            }
        }

        result_blocks
    }

    /// Create an adjusted block that only contains lines within the specified range
    fn create_adjusted_block(
        &self,
        original_block: &BlameBlock,
        start_line: usize,
        end_line: usize,
    ) -> BlameBlock {
        // If the block exactly matches the range, return a clone
        if original_block.start_line == start_line && original_block.end_line == end_line {
            return original_block.clone();
        }

        // Calculate which lines to include
        let original_start = original_block.start_line;
        let lines_to_skip = start_line - original_start;
        let lines_to_take = end_line - start_line + 1;

        // Split the content and take only the relevant lines
        let original_lines: Vec<&str> = original_block.content.split('\n').collect();
        let adjusted_lines: Vec<&str> = original_lines
            .into_iter()
            .skip(lines_to_skip)
            .take(lines_to_take)
            .collect();

        let adjusted_content = adjusted_lines.join("\n");

        BlameBlock {
            content: adjusted_content,
            blame_info: original_block.blame_info.clone(),
            start_line,
            end_line,
            line_count: lines_to_take,
        }
    }

    /// Convert blame lines to blame blocks by grouping consecutive lines with the same commit
    fn convert_lines_to_blocks(&self, blame_lines: Vec<BlameLine>) -> Vec<BlameBlock> {
        if blame_lines.is_empty() {
            return Vec::new();
        }

        let mut blocks = Vec::new();
        let mut current_block_start = 0;
        let mut current_commit_hash = &blame_lines[0].blame_info.commit_hash;

        // Process all lines in the main loop
        for (i, line) in blame_lines.iter().enumerate() {
            let is_commit_changed = line.blame_info.commit_hash != *current_commit_hash;

            if is_commit_changed {
                // Create a block for the previous group (from current_block_start to i-1)
                self.create_blame_block(&blame_lines, current_block_start, i, &mut blocks);

                // Start a new block from the current line
                current_block_start = i;
                current_commit_hash = &line.blame_info.commit_hash;
            }
        }

        // Handle the final block after the loop (from current_block_start to the end)
        self.create_blame_block(
            &blame_lines,
            current_block_start,
            blame_lines.len(),
            &mut blocks,
        );

        blocks
    }

    /// Helper function to create a blame block from a range of lines
    fn create_blame_block(
        &self,
        blame_lines: &[BlameLine],
        start_index: usize,
        end_index: usize,
        blocks: &mut Vec<BlameBlock>,
    ) {
        if start_index >= end_index || start_index >= blame_lines.len() {
            return;
        }

        // Collect content for the block
        let content: Vec<&str> = blame_lines[start_index..end_index]
            .iter()
            .map(|l| l.content.as_str())
            .collect();
        let joined_content = content.join("\n");

        // Create the block
        let start_line = blame_lines[start_index].line_number;
        let end_line = blame_lines[end_index - 1].line_number;
        let line_count = end_index - start_index;

        blocks.push(BlameBlock {
            content: joined_content,
            blame_info: blame_lines[start_index].blame_info.clone(),
            start_line,
            end_line,
            line_count,
        });
    }

    /// Get file blob hash from commit tree
    async fn get_file_blob_hash(
        &self,
        file_path: &PathBuf,
        commit: &Commit,
    ) -> Result<Option<SHA1>, GitError> {
        tracing::debug!(
            "get_file_blob_hash: commit_id={}, tree_id={}",
            commit.id,
            commit.tree_id
        );
        let tree = self.get_tree_by_hash(&commit.tree_id.to_string()).await?;

        let path_components: Vec<&str> = file_path.iter().filter_map(|s| s.to_str()).collect();

        tracing::debug!(
            "Looking for file: {:?}, path_components: {:?}",
            file_path,
            path_components
        );
        tracing::debug!("Tree has {} items", tree.tree_items.len());
        for item in &tree.tree_items {
            tracing::debug!("Tree item: name={}, mode={:?}", item.name, item.mode);
        }

        let mut current_tree = tree;

        // Navigate through directory structure
        for (i, component) in path_components.iter().enumerate() {
            if i == path_components.len() - 1 {
                // Last component should be a file
                for item in &current_tree.tree_items {
                    if item.name == *component {
                        match item.mode {
                            TreeItemMode::Blob | TreeItemMode::BlobExecutable => {
                                return Ok(Some(item.id));
                            }
                            _ => return Ok(None),
                        }
                    }
                }
                return Ok(None);
            } else {
                // Intermediate component should be a directory
                let mut found = false;
                for item in &current_tree.tree_items {
                    if item.name == *component && item.mode == TreeItemMode::Tree {
                        current_tree = self.get_tree_by_hash(&item.id.to_string()).await?;
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Ok(None);
                }
            }
        }

        Ok(None)
    }

    /// Get tree by hash
    async fn get_tree_by_hash(&self, hash: &str) -> Result<Tree, GitError> {
        let tree_id = SHA1::from_str(hash)
            .map_err(|e| GitError::InvalidHashValue(format!("Invalid tree hash: {}", e)))?;

        match self
            .mono_storage
            .get_tree_by_hash(&tree_id.to_string())
            .await
        {
            Ok(Some(tree)) => Ok(Tree::from_mega_model(tree)),
            Ok(None) => Err(GitError::ObjectNotFound("Tree not found".to_string())),
            Err(e) => Err(GitError::CustomError(format!("Failed to get tree: {}", e))),
        }
    }

    /// Get commit by hash
    async fn get_commit_by_hash(&self, hash: &str) -> Result<Option<Commit>, GitError> {
        let commit_id = SHA1::from_str(hash)
            .map_err(|e| GitError::InvalidHashValue(format!("Invalid commit hash: {}", e)))?;

        match self
            .mono_storage
            .get_commit_by_hash(&commit_id.to_string())
            .await
        {
            Ok(Some(commit)) => Ok(Some(Commit::from_mega_model(commit))),
            _ => Ok(None), // Commit not found
        }
    }

    /// Get blob content
    async fn get_blob_content(&self, blob_hash: &SHA1) -> Result<String, GitError> {
        let blob = self
            .raw_db_storage
            .get_raw_blob_by_hash(&blob_hash.to_string())
            .await
            .map_err(|e| GitError::CustomError(format!("Failed to get blob: {}", e)))?
            .ok_or_else(|| GitError::ObjectNotFound("Blob not found".to_string()))?;

        String::from_utf8(blob.data.unwrap_or_default())
            .map_err(|e| GitError::ConversionError(format!("Invalid UTF-8 in blob: {}", e)))
    }

    /// Get file blame with automatic streaming for large files (API-friendly)
    pub async fn get_file_blame_streaming_auto(
        &self,
        file_path: &str,
        ref_name: Option<&str>,
        query: BlameQuery,
    ) -> Result<BlameResult, GitError> {
        let file_path_buf = PathBuf::from(file_path);

        // Resolve the commit to analyze
        let commit = self.resolve_commit(ref_name).await?;

        // Get the file version to determine total lines (using cache)
        let current_version = self
            .get_file_version_cached(&file_path_buf, &commit)
            .await?;
        let total_lines = current_version.lines.len();

        // Determine the processing range
        let start_line = query.start_line.unwrap_or(1);
        let end_line = query.end_line.unwrap_or(total_lines);

        // Use streaming processing
        let result = self
            .get_file_blame_streaming(file_path, ref_name, start_line, end_line, None)
            .await?;

        // Calculate earliest and latest commit times from all blocks
        let (earliest_commit_time, latest_commit_time) =
            self.calculate_commit_time_range(&result.blocks);

        // Collect contributors from blocks
        let contributors = self.collect_contributors(&result.blocks).await?;

        // Return the result with blocks (pagination is already handled in get_file_blame_streaming)
        Ok(BlameResult {
            file_path: result.file_path,
            blocks: result.blocks,
            total_lines,
            page: query.page,
            page_size: query.page_size,
            earliest_commit_time,
            latest_commit_time,
            contributors,
        })
    }

    /// Get file blame with streaming support for large files
    pub async fn get_file_blame_streaming(
        &self,
        file_path: &str,
        ref_name: Option<&str>,
        start_line: usize,
        end_line: usize,
        chunk_size: Option<usize>,
    ) -> Result<BlameResult, GitError> {
        let file_path_buf = PathBuf::from(file_path);

        // Resolve the commit to analyze
        let commit = self.resolve_commit(ref_name).await?;

        // Get the file version at this commit to determine optimal chunk size (using cache)
        let current_version = self
            .get_file_version_cached(&file_path_buf, &commit)
            .await?;

        let chunk_size =
            chunk_size.unwrap_or_else(|| self.get_optimal_chunk_size(&current_version.lines));

        // Validate line range
        let actual_start = start_line.max(1);
        let actual_end = end_line.min(current_version.lines.len());

        if actual_start > actual_end {
            return Ok(BlameResult {
                file_path: file_path_buf.to_string_lossy().to_string(),
                blocks: Vec::new(),
                total_lines: current_version.lines.len(),
                page: None,
                page_size: None,
                earliest_commit_time: 0,
                latest_commit_time: 0,
                contributors: Vec::new(),
            });
        }

        // Process in chunks
        let mut all_blocks = Vec::new();

        for chunk_start in (actual_start..=actual_end).step_by(chunk_size) {
            let chunk_end = (chunk_start + chunk_size - 1).min(actual_end);

            let query = BlameQuery {
                start_line: Some(chunk_start),
                end_line: Some(chunk_end),
                page: None,
                page_size: None,
            };

            let chunk_result = self
                .get_file_blame(file_path, ref_name, Some(query))
                .await?;
            all_blocks.extend(chunk_result.blocks);
        }

        // NOTE: This function's role is ONLY to aggregate blocks from chunks.
        // Final metadata aggregation (times, contributors) is left to the caller
        // (get_file_blame_streaming_auto) to avoid redundant calculations.
        Ok(BlameResult {
            file_path: file_path.to_string(),
            blocks: all_blocks,
            total_lines: current_version.lines.len(),
            page: None,
            page_size: None,
            // These should be initialized to zero/empty, as the caller will re-calculate them
            earliest_commit_time: 0,
            latest_commit_time: 0,
            contributors: Vec::new(),
        })
    }

    /// Check if a file should be considered large based on configuration
    fn is_large_file(&self, content: &[String], file_size: Option<usize>) -> bool {
        if let Some(config) = self.config.upgrade() {
            let max_lines = config.blame.max_lines_threshold;
            let max_bytes = config.blame.get_max_size_bytes().unwrap_or(usize::MAX);

            // Check line count threshold
            if content.len() > max_lines {
                return true;
            }

            // Check file size threshold if provided
            if let Some(size) = file_size
                && size > max_bytes
            {
                return true;
            }
        } else {
            tracing::warn!(
                "Could not upgrade config to check if file is large, defaulting to false."
            );
            return false;
        }

        false
    }

    /// Get optimal chunk size for a file based on its characteristics
    fn get_optimal_chunk_size(&self, content: &[String]) -> usize {
        let default_chunk_size = if let Some(config) = self.config.upgrade() {
            config.blame.default_chunk_size
        } else {
            tracing::warn!("Could not upgrade config for chunk size, using default value 100.");
            100
        };

        if self.is_large_file(content, None) {
            // For large files, use smaller chunks to reduce memory usage
            default_chunk_size.min(50)
        } else {
            // For normal files, use larger chunks for efficiency
            default_chunk_size
        }
    }

    /// Get file version from cache or storage
    async fn get_file_version_cached(
        &self,
        file_path: &PathBuf,
        commit: &Commit,
    ) -> Result<FileVersion, GitError> {
        let caching_enabled = if let Some(config) = self.config.upgrade() {
            config.blame.enable_caching
        } else {
            tracing::warn!("Could not upgrade config to check caching policy, disabling cache.");
            false
        };

        if caching_enabled {
            let cache_key = format!("{}:{}", commit.id, file_path.display());

            if let Some(cached_version) = self.cache.file_versions.read().await.get(&cache_key) {
                return Ok((**cached_version).clone());
            }
        }

        // Get from storage
        let version = self.get_file_version(file_path, commit).await?;

        // Cache the result if caching is enabled
        if caching_enabled {
            let cache_key = format!("{}:{}", commit.id, file_path.display());
            let mut cache = self.cache.file_versions.write().await;
            cache.insert(cache_key, Arc::new(version.clone()));
        }

        Ok(version)
    }

    /// Get commit from cache or storage
    async fn get_commit_cached(&self, commit_hash: &SHA1) -> Result<Option<Commit>, GitError> {
        let caching_enabled = if let Some(config) = self.config.upgrade() {
            config.blame.enable_caching
        } else {
            tracing::warn!("Could not upgrade config to check caching policy, disabling cache.");
            false
        };

        if caching_enabled {
            // Try to get from cache first
            let cache_key = commit_hash.to_string();
            {
                let cache = self.cache.commits.read().await;
                if let Some(cached_commit) = cache.get(&cache_key) {
                    return Ok(Some((**cached_commit).clone()));
                }
            }
        }

        // Get from storage
        let commit = self.get_commit_by_hash(&commit_hash.to_string()).await?;

        // Cache the result if caching is enabled and commit exists
        if caching_enabled && let Some(ref commit_obj) = commit {
            let cache_key = commit_hash.to_string();
            let mut cache = self.cache.commits.write().await;
            cache.insert(cache_key, Arc::new(commit_obj.clone()));
        }

        Ok(commit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::base_storage::StorageConnector;
    use git_internal::internal::object::blob::Blob;
    use git_internal::internal::object::commit::Commit;
    use git_internal::internal::object::signature::{Signature, SignatureType};
    use git_internal::internal::object::tree::{Tree, TreeItem, TreeItemMode};
    use serde_json;

    /// Test case based on a file history with three users, verifying block aggregation,
    /// time metadata, and the new username-based contributor identity.
    #[tokio::test]
    async fn test_blame_service_full_validation() {
        // Create a temporary directory and test storage for isolation.
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
        let storage = crate::tests::test_storage(temp_dir.path()).await;

        // Define test users (Name, Email, Time)
        // NOTE: We use Name here to simulate the 'matched_username' we expect from the DB.
        let users = [
            ("Bob", "bob@example.com", 1758153600), // Commit 1 (Earliest)
            ("Alice", "alice@example.com", 1758240000), // Commit 2
            ("Tony", "tony@example.com", 1758326400), // Commit 3 (Latest)
        ];

        // Content versions for 'app.conf' (Final file has 6 lines)
        let content_v1 = r#"app_name = "MegaApp"
version = "1.0"
log_level = "info"
debug_mode = true
api_key = "initial_key_v1"
"#;
        let content_v2 = r#"app_name = "MegaApp"
version = "1.0"
log_level = "warn"
debug_mode = true
api_key = "intermediate_key_v2"
"#;
        let content_v3 = r#"app_name = "MegaApp"
version = "1.0"
log_level = "warn"
debug_mode = true
api_key = "final_key_v3"
enable_https = true
"#;

        // --- 1. Create Git Objects and History Chain ---
        let blob1 = Blob::from_content(content_v1);
        let blob2 = Blob::from_content(content_v2);
        let blob3 = Blob::from_content(content_v3);

        // Helper to create Signatures (Author = Committer in this setup)
        let create_sig = |user: &(&str, &str, i64)| Signature {
            signature_type: SignatureType::Author,
            name: user.0.to_string(), // Git Author Name (used in legacy systems)
            email: user.1.to_string(),
            timestamp: user.2 as usize,
            timezone: "+0800".to_string(),
        };

        let author1 = create_sig(&users[0]);
        let author2 = create_sig(&users[1]);
        let author3 = create_sig(&users[2]);

        let committer1 = author1.clone();
        let committer2 = author2.clone();
        let committer3 = author3.clone();

        let file_name = "app.conf";

        // Create Trees
        let tree1 = Tree::from_tree_items(vec![TreeItem::new(
            TreeItemMode::Blob,
            blob1.id,
            file_name.to_string(),
        )])
        .unwrap();
        let tree2 = Tree::from_tree_items(vec![TreeItem::new(
            TreeItemMode::Blob,
            blob2.id,
            file_name.to_string(),
        )])
        .unwrap();
        let tree3 = Tree::from_tree_items(vec![TreeItem::new(
            TreeItemMode::Blob,
            blob3.id,
            file_name.to_string(),
        )])
        .unwrap();

        // Create Commits
        let commit1 = Commit::new(
            author1,
            committer1,
            tree1.id,
            vec![],
            "feat: initial config by Bob",
        );
        let commit2 = Commit::new(
            author2,
            committer2,
            tree2.id,
            vec![commit1.id],
            "feat: update log level by Alice",
        );
        let commit3 = Commit::new(
            author3,
            committer3,
            tree3.id,
            vec![commit2.id],
            "refactor: finalize config by Tony",
        );

        // Save objects (Blobs, Trees, Commits, Ref)
        storage
            .app_service
            .mono_storage
            .save_mega_blobs(vec![&blob1, &blob2, &blob3], &commit3.id.to_string())
            .await
            .expect("Failed to save blobs");

        use callisto::mega_tree;
        let save_trees: Vec<mega_tree::ActiveModel> = vec![tree1, tree2, tree3.clone()]
            .into_iter()
            .map(|tree| {
                let mut tree_model: mega_tree::Model = tree.into_mega_model();
                tree_model.commit_id = "test".to_string();
                tree_model.into()
            })
            .collect();
        storage
            .app_service
            .mono_storage
            .batch_save_model(save_trees)
            .await
            .expect("Failed to save trees");
        storage
            .app_service
            .mono_storage
            .save_mega_commits(vec![commit1.clone(), commit2.clone(), commit3.clone()])
            .await
            .expect("Failed to save commits");
        storage
            .app_service
            .mono_storage
            .save_ref(
                "/",
                None,
                &commit3.id.to_string(),
                &tree3.id.to_string(),
                false,
            )
            .await
            .expect("Failed to save HEAD ref");

        // --- 2. Add Commit Binding Data (Simulating successful user matching) ---
        // This sets the 'matched_username' (Bob, Alice, Tony) in the commit_auths table.
        storage
            .app_service
            .mono_storage
            .commit_binding_storage()
            .upsert_binding(
                &commit1.id.to_string(),
                users[0].1,
                Some(users[0].0.to_string()),
                false,
            ) // Bob
            .await
            .expect("Failed to save commit binding for user 1");
        storage
            .app_service
            .mono_storage
            .commit_binding_storage()
            .upsert_binding(
                &commit2.id.to_string(),
                users[1].1,
                Some(users[1].0.to_string()),
                false,
            ) // Alice
            .await
            .expect("Failed to save commit binding for user 2");
        storage
            .app_service
            .mono_storage
            .commit_binding_storage()
            .upsert_binding(
                &commit3.id.to_string(),
                users[2].1,
                Some(users[2].0.to_string()),
                false,
            ) // Tony
            .await
            .expect("Failed to save commit binding for user 3");

        // --- 3. Act: Call the service ---
        let blame_service = BlameService::new(Arc::new(storage.clone()));
        let blame_result = blame_service
            .get_file_blame(file_name, None, None)
            .await
            .expect("Failed to get blame result");

        // --- JSON PRINTING (for debugging/visual confirmation) ---
        let json_output = serde_json::to_string_pretty(&blame_result)
            .expect("Failed to serialize BlameResult to JSON");
        println!(
            "\n=== Blame Result JSON Output (app.conf) ===\n{}",
            json_output
        );

        // --- 4. Assert: Full Validation ---

        // A. Global Metadata Assertions
        assert_eq!(blame_result.total_lines, 6, "Total lines must be 6.");
        assert_eq!(
            blame_result.blocks.len(),
            4,
            "Should be 4 aggregated blocks."
        );
        assert_eq!(
            blame_result.earliest_commit_time, users[0].2,
            "Earliest time must be Bob's commit."
        );
        assert_eq!(
            blame_result.latest_commit_time, users[2].2,
            "Latest time must be Tony's commit."
        );

        // B. Block Aggregation & Identity Assertions (Verifying new username fields)
        let block1 = &blame_result.blocks[0]; // L1-L2, Bob
        assert_eq!(block1.start_line, 1);
        assert_eq!(block1.end_line, 2);
        assert_eq!(block1.blame_info.author_username, Some("Bob".to_string()));
        assert_eq!(
            block1.blame_info.committer_username,
            Some("Bob".to_string())
        );
        assert_eq!(block1.blame_info.author_email, users[0].1);

        let block2 = &blame_result.blocks[1]; // L3, Alice
        assert_eq!(block2.start_line, 3);
        assert_eq!(block2.end_line, 3);
        assert_eq!(block2.blame_info.author_username, Some("Alice".to_string()));

        let block4 = &blame_result.blocks[3]; // L5-L6, Tony
        assert_eq!(block4.start_line, 5);
        assert_eq!(block4.end_line, 6);
        assert_eq!(block4.blame_info.author_username, Some("Tony".to_string()));
        assert_eq!(
            block4.blame_info.committer_username,
            Some("Tony".to_string())
        );
        assert_eq!(block4.blame_info.author_email, users[2].1);

        // C. Contributor Aggregation Assertions
        assert_eq!(
            blame_result.contributors.len(),
            3,
            "Total contributors must be 3."
        );

        let contributor_map: HashMap<String, Contributor> = blame_result
            .contributors
            .into_iter()
            .map(|c| (c.email.clone(), c))
            .collect();

        // Verify Bob (3 lines, Earliest time)
        let bob_contributor = contributor_map.get(users[0].1).expect("Bob not found");
        assert_eq!(
            bob_contributor.total_lines, 3,
            "Bob should have 3 lines (L1, L2, L4)."
        );
        assert_eq!(
            bob_contributor.username,
            Some("Bob".to_string()),
            "Bob's username should be set."
        );
        assert_eq!(
            bob_contributor.last_commit_time, users[0].2,
            "Bob's last commit time should be his only commit time."
        );

        // Verify Tony (2 lines, Latest time)
        let tony_contributor = contributor_map.get(users[2].1).expect("Tony not found");
        assert_eq!(
            tony_contributor.total_lines, 2,
            "Tony should have 2 lines (L5, L6)."
        );
    }

    #[tokio::test]
    async fn test_block_level_pagination() {
        // Setup test environment
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
        let storage = crate::tests::test_storage(temp_dir.path()).await;

        // Create a simple test file for pagination testing
        let file_name = "test_pagination.txt";
        let content = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\n";

        // Create blob and tree
        let blob = Blob::from_content(content);
        let tree_item = TreeItem::new(TreeItemMode::Blob, blob.id, file_name.to_string());
        let tree = Tree::from_tree_items(vec![tree_item]).unwrap();

        // Create commit
        let signature = Signature {
            signature_type: SignatureType::Author,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            timestamp: 1758153600,
            timezone: "+0800".to_string(),
        };

        let commit = Commit::new(
            signature.clone(),
            signature,
            tree.id,
            vec![],
            "Test commit for pagination",
        );

        // Save objects to storage
        storage
            .app_service
            .mono_storage
            .save_mega_blobs(vec![&blob], &commit.id.to_string())
            .await
            .expect("Failed to save blob");

        use callisto::mega_tree;
        let save_trees: Vec<mega_tree::ActiveModel> = vec![tree.clone()]
            .into_iter()
            .map(|tree| {
                let mut tree_model: mega_tree::Model = tree.into_mega_model();
                tree_model.commit_id = "test".to_string();
                tree_model.into()
            })
            .collect();
        storage
            .app_service
            .mono_storage
            .batch_save_model(save_trees)
            .await
            .expect("Failed to save trees");
        storage
            .app_service
            .mono_storage
            .save_mega_commits(vec![commit.clone()])
            .await
            .expect("Failed to save commits");
        storage
            .app_service
            .mono_storage
            .save_ref(
                "/",
                None,
                &commit.id.to_string(),
                &tree.id.to_string(),
                false,
            )
            .await
            .expect("Failed to save HEAD ref");

        let blame_service = BlameService::new(Arc::new(storage.clone()));

        // Test 1: Page-based pagination
        let query = BlameQuery {
            page: Some(1),
            page_size: Some(2),
            start_line: None,
            end_line: None,
        };

        let result = blame_service
            .get_file_blame(file_name, None, Some(query))
            .await
            .expect("Failed to get blame result with page pagination");

        // Should return first 2 lines (which might span multiple blocks)
        assert!(!result.blocks.is_empty(), "Should have at least one block");
        assert_eq!(
            result.blocks[0].start_line, 1,
            "First block should start at line 1"
        );

        // Test 2: Line range pagination
        let query = BlameQuery {
            page: None,
            page_size: None,
            start_line: Some(3),
            end_line: Some(5),
        };

        let result = blame_service
            .get_file_blame(file_name, None, Some(query))
            .await
            .expect("Failed to get blame result with line range pagination");

        // Should return lines 3-5
        assert!(!result.blocks.is_empty(), "Should have at least one block");
        let first_block = &result.blocks[0];
        let last_block = &result.blocks[result.blocks.len() - 1];

        assert!(
            first_block.start_line >= 3,
            "First block should start at or after line 3"
        );
        assert!(
            last_block.end_line <= 5,
            "Last block should end at or before line 5"
        );

        // Test 3: Single line pagination
        let query = BlameQuery {
            page: None,
            page_size: None,
            start_line: Some(4),
            end_line: Some(4),
        };

        let result = blame_service
            .get_file_blame(file_name, None, Some(query))
            .await
            .expect("Failed to get blame result with single line pagination");

        // Should return exactly line 4
        assert_eq!(
            result.blocks.len(),
            1,
            "Should have exactly one block for single line"
        );
        let block = &result.blocks[0];
        assert_eq!(block.start_line, 4, "Block should start at line 4");
        assert_eq!(block.end_line, 4, "Block should end at line 4");
        assert_eq!(block.line_count, 1, "Block should contain exactly 1 line");

        println!("All pagination tests passed!");
    }

    #[tokio::test]
    async fn test_block_adjustment_logic() {
        // Setup test environment - reuse the existing test setup
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
        let storage = crate::tests::test_storage(temp_dir.path()).await;
        let file_name = "app.conf"; // Use the same file as the main test

        // Use the same setup as the main test to ensure we have multi-line blocks
        let users = [
            ("Bob", "bob@example.com", 1758153600),
            ("Alice", "alice@example.com", 1758240000),
            ("Tony", "tony@example.com", 1758326400),
        ];

        let content_v3 = r#"app_name = "MegaApp"
version = "1.0"
log_level = "warn"
debug_mode = true
api_key = "final_key_v3"
enable_https = true
"#;

        // Create the final blob and commit (simplified setup)
        let blob = Blob::from_content(content_v3);
        let tree_item = TreeItem::new(TreeItemMode::Blob, blob.id, file_name.to_string());
        let tree = Tree::from_tree_items(vec![tree_item]).unwrap();

        let signature = Signature {
            signature_type: SignatureType::Author,
            name: users[2].0.to_string(),
            email: users[2].1.to_string(),
            timestamp: users[2].2 as usize,
            timezone: "+0800".to_string(),
        };

        let commit = Commit::new(signature.clone(), signature, tree.id, vec![], "Test commit");

        // Save objects
        storage
            .app_service
            .mono_storage
            .save_mega_blobs(vec![&blob], &commit.id.to_string())
            .await
            .expect("Failed to save blob");

        use callisto::mega_tree;
        let save_trees: Vec<mega_tree::ActiveModel> = vec![tree.clone()]
            .into_iter()
            .map(|tree| {
                let mut tree_model: mega_tree::Model = tree.into_mega_model();
                tree_model.commit_id = "test".to_string();
                tree_model.into()
            })
            .collect();
        storage
            .app_service
            .mono_storage
            .batch_save_model(save_trees)
            .await
            .expect("Failed to save trees");
        storage
            .app_service
            .mono_storage
            .save_mega_commits(vec![commit.clone()])
            .await
            .expect("Failed to save commits");
        storage
            .app_service
            .mono_storage
            .save_ref(
                "/",
                None,
                &commit.id.to_string(),
                &tree.id.to_string(),
                false,
            )
            .await
            .expect("Failed to save HEAD ref");

        let blame_service = BlameService::new(Arc::new(storage.clone()));

        // Get all blocks first
        let full_result = blame_service
            .get_file_blame(file_name, None, None)
            .await
            .expect("Failed to get full blame result");

        println!("Full result has {} blocks", full_result.blocks.len());
        for (i, block) in full_result.blocks.iter().enumerate() {
            println!(
                "Block {}: lines {}-{}, content: {:?}",
                i,
                block.start_line,
                block.end_line,
                block.content.lines().collect::<Vec<_>>()
            );
        }

        // Test block adjustment when pagination cuts through a block
        // Find a block that spans multiple lines
        let multi_line_block = full_result
            .blocks
            .iter()
            .find(|block| block.line_count > 1)
            .expect("Should have at least one multi-line block");

        let start_line = multi_line_block.start_line + 1; // Start from middle of block
        let end_line = multi_line_block.end_line;

        let query = BlameQuery {
            page: None,
            page_size: None,
            start_line: Some(start_line),
            end_line: Some(end_line),
        };

        let result = blame_service
            .get_file_blame(file_name, None, Some(query))
            .await
            .expect("Failed to get blame result with block adjustment");

        // Verify the adjusted block
        let adjusted_block = &result.blocks[0];
        assert_eq!(
            adjusted_block.start_line, start_line,
            "Adjusted block should start at requested line"
        );
        assert_eq!(
            adjusted_block.end_line, end_line,
            "Adjusted block should end at requested line"
        );

        // Verify content is properly adjusted
        let expected_line_count = end_line - start_line + 1;
        assert_eq!(
            adjusted_block.line_count, expected_line_count,
            "Adjusted block should have correct line count"
        );

        println!("Block adjustment test passed!");
    }
}
