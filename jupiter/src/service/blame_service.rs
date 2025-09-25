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
    BlameCandidate, BlameInfo, BlameLine, BlameQuery, BlameResult, FileVersion, LargeFileConfig,
    LineAttribution,
};
use crate::storage::{
    mono_storage::MonoStorage, raw_db_storage::RawDbStorage, user_storage::UserStorage, Storage,
};
use common::config::Config;
use mercury::errors::GitError;
use mercury::hash::SHA1;
use neptune::{compute_diff, DiffOperation};

use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::{Tree, TreeItemMode};
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
    user_storage: Arc<UserStorage>,
    cache: Arc<BlameCache>,
    config: Weak<Config>,
}

impl BlameService {
    /// Create a new BlameService instance using global configuration
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            mono_storage: Arc::new(storage.mono_storage()),
            raw_db_storage: Arc::new(storage.raw_db_storage()),
            user_storage: Arc::new(storage.user_storage()),
            cache: Arc::new(BlameCache::new()),
            config: storage.config.clone(),
        }
    }

    /// Create a mock instance for testing
    pub fn mock() -> Self {
        let storage = Arc::new(Storage::mock());
        Self::new(storage)
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

        // Apply pagination if requested
        let final_lines = self.apply_pagination(blame_lines, &query);

        Ok(BlameResult {
            file_path: file_path.to_string_lossy().to_string(),
            lines: final_lines,
            total_lines: current_version.lines.len(),
            page: query.as_ref().and_then(|q| q.page),
            page_size: query.as_ref().and_then(|q| q.page_size),
        })
    }

    /// Resolve commit from reference name or use HEAD (with caching)
    async fn resolve_commit(&self, ref_name: Option<&str>) -> Result<Commit, GitError> {
        tracing::debug!("resolve_commit: ref_name={:?}", ref_name);

        // 🚀 Step 1: Attempt to fetch from cache
        let cache_key = ref_name.unwrap_or("HEAD").to_string();

        {
            let cache = self.cache.commits.read().await;
            if let Some(cached_commit) = cache.get(&cache_key) {
                tracing::debug!("Cache hit for commit: {}", cache_key);
                return Ok((**cached_commit).clone());
            }
        }

        // 🔄 Step 2: Cache miss, fetch from storage
        tracing::debug!(
            "Cache miss for commit: {}, resolving from storage",
            cache_key
        );
        let commit = match ref_name {
            Some(ref_str) => {
                // Try to get commit by hash first
                if let Ok(Some(commit)) = self.get_commit_by_hash(ref_str).await {
                    tracing::debug!("Found commit by hash: {}", commit.id);
                    Ok(commit)
                } else {
                    // Try to resolve by branch name using get_ref_by_name
                    let full_ref_name = if ref_str.starts_with("refs/") {
                        ref_str.to_string()
                    } else {
                        format!("refs/heads/{}", ref_str)
                    };
                    tracing::debug!("Trying to resolve ref: {}", full_ref_name);
                    match self.mono_storage.get_ref_by_name(&full_ref_name).await {
                        Ok(Some(commit_id)) => self
                            .get_commit_by_hash(&commit_id.ref_commit_hash)
                            .await?
                            .ok_or_else(|| GitError::ObjectNotFound(ref_str.to_string())),
                        Ok(None) => {
                            // Fallback to default branch
                            tracing::debug!(
                                "Ref not found by name, falling back to get_ref with root path"
                            );
                            match self.mono_storage.get_ref("/").await {
                                Ok(Some(commit_id)) => {
                                    tracing::debug!(
                                        "Found commit via fallback: {}",
                                        commit_id.ref_commit_hash
                                    );
                                    self.get_commit_by_hash(&commit_id.ref_commit_hash)
                                        .await?
                                        .ok_or_else(|| {
                                            GitError::ObjectNotFound(ref_str.to_string())
                                        })
                                }
                                Ok(None) => {
                                    tracing::debug!("No commit found via fallback");
                                    Err(GitError::ObjectNotFound(ref_str.to_string()))
                                }
                                Err(e) => Err(GitError::CustomError(format!(
                                    "Failed to resolve reference: {}",
                                    e
                                ))),
                            }
                        }
                        Err(e) => Err(GitError::CustomError(format!(
                            "Failed to resolve reference: {}",
                            e
                        ))),
                    }
                }
            }
            None => {
                // Use root path to get the default reference
                match self.mono_storage.get_ref("/").await {
                    Ok(Some(commit_id)) => self
                        .get_commit_by_hash(&commit_id.ref_commit_hash)
                        .await?
                        .ok_or_else(|| GitError::ObjectNotFound("HEAD".to_string())),
                    Ok(None) => Err(GitError::ObjectNotFound(
                        "No HEAD or main branch found".to_string(),
                    )),
                    Err(e) => Err(GitError::CustomError(format!(
                        "Failed to resolve HEAD: {}",
                        e
                    ))),
                }
            }
        };

        // 💾 Step 3: Store in cache
        let resolved_commit = commit?;
        {
            let mut cache = self.cache.commits.write().await;
            cache.insert(cache_key.clone(), Arc::new(resolved_commit.clone()));
            tracing::debug!("Cached commit: {}", cache_key);
        }

        Ok(resolved_commit)
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

    /// Get user avatar URL from database, fallback to gravatar if not found
    async fn get_user_avatar_url(&self, email: &str, name: &str) -> String {
        // First try to find user by email
        if let Ok(Some(user)) = self.user_storage.find_user_by_email(email).await {
            return user.avatar_url;
        }

        // Then try to find user by name
        if let Ok(Some(user)) = self.user_storage.find_user_by_name(name).await {
            return user.avatar_url;
        }

        "".to_string()
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

            // Get real user avatar URL from database
            let author_avatar_url = self
                .get_user_avatar_url(&commit.author.email, &commit.author.name)
                .await;

            let blame_info = BlameInfo {
                commit_hash: commit_hash_str.clone(),
                commit_short_id: commit_hash_str.chars().take(7).collect(),
                author_name: commit.author.name.clone(),
                author_email: commit.author.email.clone(),
                author_time,
                committer_name: commit.committer.name.clone(),
                committer_email: commit.committer.email.clone(),
                committer_time: commit.committer.timestamp as i64,
                commit_message: commit.message.clone(),
                commit_summary: commit.message.lines().next().unwrap_or("").to_string(),
                original_line_number: attr.line_number_in_commit,
                author_avatar_url,
                commit_detail_url: format!("/commit/{}", commit_hash_str),
                author_profile_url: format!("/people/{}", commit.author.name),
            };

            blame_lines.push(BlameLine {
                line_number: attr.line_number,
                content: attr.content,
                blame_info,
            });
        }

        Ok(blame_lines)
    }

    /// Apply pagination to blame results
    fn apply_pagination(
        &self,
        blame_lines: Vec<BlameLine>,
        query: &Option<BlameQuery>,
    ) -> Vec<BlameLine> {
        if let Some(q) = query {
            if let (Some(page), Some(page_size)) = (q.page, q.page_size) {
                if page == 0 || page_size == 0 {
                    return Vec::new();
                }
                let start = (page - 1) * page_size;

                if start >= blame_lines.len() {
                    return Vec::new();
                }

                let end = (start + page_size).min(blame_lines.len());

                blame_lines[start..end].to_vec()
            } else {
                let start = q.start_line.unwrap_or(1).saturating_sub(1);
                let end = q
                    .end_line
                    .unwrap_or(blame_lines.len())
                    .min(blame_lines.len());

                if start < blame_lines.len() {
                    blame_lines[start..end].to_vec()
                } else {
                    Vec::new()
                }
            }
        } else {
            blame_lines
        }
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
            Ok(Some(tree)) => Ok(tree.into()),
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
            Ok(Some(commit)) => Ok(Some(commit.into())),
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

        // Apply pagination
        let final_lines = if query.page.is_some() || query.page_size.is_some() {
            self.apply_pagination(result.lines, &Some(query.clone()))
        } else {
            result.lines
        };

        Ok(BlameResult {
            file_path: result.file_path,
            lines: final_lines,
            total_lines,
            page: query.page,
            page_size: query.page_size,
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
                lines: Vec::new(),
                total_lines: current_version.lines.len(),
                page: None,
                page_size: None,
            });
        }

        // Process in chunks
        let mut all_blame_lines = Vec::new();

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
            all_blame_lines.extend(chunk_result.lines);
        }

        Ok(BlameResult {
            file_path: file_path.to_string(),
            lines: all_blame_lines,
            total_lines: current_version.lines.len(),
            page: None,
            page_size: None,
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
            if let Some(size) = file_size {
                if size > max_bytes {
                    return true;
                }
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
        if caching_enabled {
            if let Some(ref commit_obj) = commit {
                let cache_key = commit_hash.to_string();
                let mut cache = self.cache.commits.write().await;
                cache.insert(cache_key, Arc::new(commit_obj.clone()));
            }
        }

        Ok(commit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::base_storage::StorageConnector;
    use mercury::internal::object::blob::Blob;
    use mercury::internal::object::commit::Commit;
    use mercury::internal::object::signature::{Signature, SignatureType};
    use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};
    use serde_json;

    /// A comprehensive blame test with a commit history of three users.
    #[tokio::test]
    async fn test_blame_service_with_three_users() {
        // Create a temporary directory and test storage for isolation.
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
        let storage = crate::tests::test_storage(temp_dir.path()).await;

        // Define test data for three users.
        let users = [
            (
                "Zhang Wei",
                "zhang.wei@example.com",
                "https://avatar.example.com/zhangwei",
            ),
            (
                "Li Na",
                "li.na@example.com",
                "https://avatar.example.com/lina",
            ),
            (
                "Wang Fang",
                "wang.fang@example.com",
                "https://avatar.example.com/wangfang",
            ),
        ];

        let content_v1 = r#"app_name = "MegaApp"
version = "1.0"
log_level = "info"
debug_mode = true
api_key = "initial_key_v1"
"#;

        let content_v2 = r#"app_name = "MegaApp"
    version = "1.0"
log_level = "warn"

api_key = "intermediate_key_v2"
"#;

        let content_v3 = r#"app_name = "MegaApp"
    version = "1.0"

log_level = "warn"
api_key = "final_key_v3"
enable_https = true
"#;

        // Create Blob objects from content.
        let blob1 = Blob::from_content(content_v1);
        let blob2 = Blob::from_content(content_v2);
        let blob3 = Blob::from_content(content_v3);

        // Create signatures for each user.
        let author1 = Signature {
            signature_type: SignatureType::Author,
            name: users[0].0.to_string(),
            email: users[0].1.to_string(),
            timestamp: 1758153600,
            timezone: "+0800".to_string(),
        };
        let committer1 = author1.clone();

        let author2 = Signature {
            signature_type: SignatureType::Author,
            name: users[1].0.to_string(),
            email: users[1].1.to_string(),
            timestamp: 1758240000,
            timezone: "+0800".to_string(),
        };
        let committer2 = author2.clone();

        let author3 = Signature {
            signature_type: SignatureType::Author,
            name: users[2].0.to_string(),
            email: users[2].1.to_string(),
            timestamp: 1758326400,
            timezone: "+0800".to_string(),
        };
        let committer3 = author3.clone();

        // Create TreeItem objects linking blobs to file names.
        let tree_item1 = TreeItem::new(TreeItemMode::Blob, blob1.id, "app.conf".to_string());
        let tree_item2 = TreeItem::new(TreeItemMode::Blob, blob2.id, "app.conf".to_string());
        let tree_item3 = TreeItem::new(TreeItemMode::Blob, blob3.id, "app.conf".to_string());

        // Create Tree objects from TreeItems.
        let tree1 = Tree::from_tree_items(vec![tree_item1]).unwrap();
        let tree2 = Tree::from_tree_items(vec![tree_item2]).unwrap();
        let tree3 = Tree::from_tree_items(vec![tree_item3]).unwrap();

        // Create Commit objects to form a history chain.
        let commit1 = Commit::new(
            author1,
            committer1,
            tree1.id,
            vec![], // Initial commit, no parent.
            "feat: initial application configuration",
        );

        let commit2 = Commit::new(
            author2,
            committer2,
            tree2.id,
            vec![commit1.id], // Parent is commit1.
            "feat: update database config and add user credentials",
        );

        let commit3 = Commit::new(
            author3,
            committer3,
            tree3.id,
            vec![commit2.id], // Parent is commit2.
            "refactor: cleanup config and prepare for production",
        );

        // --- Test Setup Information ---
        println!("\n=== Test Data Construction Complete ===");
        println!("User 1 - {}: {}", users[0].0, users[0].1);
        println!("Commit: {} (Initial config)", commit1.id);
        println!("Time: {}", 1758153600);

        println!("\nUser 2 - {}: {}", users[1].0, users[1].1);
        println!("Commit: {} (Update database config)", commit2.id);
        println!("Parent: {}", commit1.id);
        println!("Time: {}", 1758240000);

        println!("\nUser 3 - {}: {}", users[2].0, users[2].1);
        println!("Commit: {} (Refactor config)", commit3.id);
        println!("Parent: {}", commit2.id);
        println!("Time: {}", 1758326400);

        storage
            .app_service
            .mono_storage
            .save_mega_blobs(vec![&blob1], &commit1.id.to_string())
            .await
            .expect("Failed to save blob1");
        storage
            .app_service
            .mono_storage
            .save_mega_blobs(vec![&blob2], &commit2.id.to_string())
            .await
            .expect("Failed to save blob2");
        storage
            .app_service
            .mono_storage
            .save_mega_blobs(vec![&blob3], &commit3.id.to_string())
            .await
            .expect("Failed to save blob3");

        use callisto::mega_tree;
        let save_trees: Vec<mega_tree::ActiveModel> =
            vec![tree1.clone(), tree2.clone(), tree3.clone()]
                .into_iter()
                .map(|tree| {
                    let mut tree_model: mega_tree::Model = tree.into();
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
            .save_mega_commits(vec![commit1.clone()])
            .await
            .expect("Failed to save commit1");
        storage
            .app_service
            .mono_storage
            .save_mega_commits(vec![commit2.clone()])
            .await
            .expect("Failed to save commit2");
        storage
            .app_service
            .mono_storage
            .save_mega_commits(vec![commit3.clone()])
            .await
            .expect("Failed to save commit3");

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
            .expect("Failed to save HEAD reference");

        // --- Act: Create the service and call the method under test. ---
        let blame_service = BlameService::new(Arc::new(storage.clone()));

        let blame_result = blame_service
            .get_file_blame("app.conf", None, None)
            .await
            .expect("Failed to get blame result");

        // --- Assert: Print the result for visual verification. ---
        let json_output = serde_json::to_string_pretty(&blame_result).unwrap();
        println!("\n=== Blame Result (JSON Format) ===");
        println!("{}", json_output);

        // Perform high-level checks to ensure the process was successful.
        assert!(
            !blame_result.lines.is_empty(),
            "Blame result should not be empty"
        );
        assert_eq!(
            blame_result.file_path, "app.conf",
            "File path should be correct"
        );
    }
}
