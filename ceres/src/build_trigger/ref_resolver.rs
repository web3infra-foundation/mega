//! Git Reference Resolution Service
//!
//! Provides unified resolution of git references (branches, tags, commits) to commit hashes.
//! This service handles:
//! - Branch name → latest commit on branch
//! - Tag name → tagged commit
//! - Commit hash → validated commit hash
//! - Default resolution to "main" branch when no ref specified
//! - Ambiguous ref handling (branch and tag with same name - prefer branch)

use common::errors::MegaError;
use jupiter::storage::Storage;

/// Result of ref resolution containing the commit hash and metadata
#[derive(Debug, Clone)]
pub struct ResolvedRef {
    /// The resolved commit hash
    pub commit_hash: String,
    /// The original ref name (branch/tag name, or commit hash)
    pub ref_name: String,
    /// The type of ref: "branch", "tag", or "commit"
    pub ref_type: RefType,
}

/// Type of git reference
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefType {
    Branch,
    Tag,
    Commit,
}

impl RefType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RefType::Branch => "branch",
            RefType::Tag => "tag",
            RefType::Commit => "commit",
        }
    }
}

/// Service for resolving git references to commit hashes
pub struct RefResolver {
    storage: Storage,
}

impl RefResolver {
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    /// Resolve a git reference to a commit hash with metadata.
    ///
    /// Resolution order:
    /// 1. If ref is None, resolve to "main" branch
    /// 2. Try as branch name (refs/heads/{ref})
    /// 3. Try as tag name (refs/tags/{ref})
    /// 4. Try as commit hash (validate it exists)
    /// 5. Return error if not found
    ///
    /// For ambiguous refs (both branch and tag exist), prefer branch.
    pub async fn resolve(&self, ref_name: Option<&str>) -> Result<ResolvedRef, MegaError> {
        // Handle default case: no ref specified → use "main"
        let ref_name = ref_name.unwrap_or("main");
        tracing::debug!("Resolving ref: {}", ref_name);

        // Try to resolve as branch first
        if let Some(resolved) = self.try_resolve_as_branch(ref_name).await? {
            return Ok(resolved);
        }

        // Try to resolve as tag
        if let Some(resolved) = self.try_resolve_as_tag(ref_name).await? {
            return Ok(resolved);
        }

        // Try to resolve as commit hash
        if let Some(resolved) = self.try_resolve_as_commit(ref_name).await? {
            return Ok(resolved);
        }

        // Not found
        Err(MegaError::Other(format!(
            "[code:404] Reference not found: '{}' (not a branch, tag, or commit)",
            ref_name
        )))
    }

    /// Try to resolve ref as a branch name
    async fn try_resolve_as_branch(
        &self,
        ref_name: &str,
    ) -> Result<Option<ResolvedRef>, MegaError> {
        let full_ref_name = format!("refs/heads/{}", ref_name);

        if let Some(branch_ref) = self
            .storage
            .mono_storage()
            .get_ref_by_name(&full_ref_name)
            .await?
        {
            return Ok(Some(ResolvedRef {
                commit_hash: branch_ref.ref_commit_hash,
                ref_name: ref_name.to_string(),
                ref_type: RefType::Branch,
            }));
        }

        Ok(None)
    }

    /// Try to resolve ref as a tag name
    async fn try_resolve_as_tag(&self, ref_name: &str) -> Result<Option<ResolvedRef>, MegaError> {
        let full_ref_name = format!("refs/tags/{}", ref_name);

        if let Some(tag_ref) = self
            .storage
            .mono_storage()
            .get_ref_by_name(&full_ref_name)
            .await?
        {
            return Ok(Some(ResolvedRef {
                commit_hash: tag_ref.ref_commit_hash,
                ref_name: ref_name.to_string(),
                ref_type: RefType::Tag,
            }));
        }

        Ok(None)
    }

    /// Try to resolve ref as a commit hash (validate it exists)
    async fn try_resolve_as_commit(
        &self,
        ref_name: &str,
    ) -> Result<Option<ResolvedRef>, MegaError> {
        // Check if it looks like a commit hash (hex string, typically 40 chars but can be shorter)
        if !ref_name.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(None);
        }

        // Validate the commit exists in the database
        if let Some(_commit) = self
            .storage
            .mono_storage()
            .get_commit_by_hash(ref_name)
            .await?
        {
            return Ok(Some(ResolvedRef {
                commit_hash: ref_name.to_string(),
                ref_name: ref_name.to_string(),
                ref_type: RefType::Commit,
            }));
        }

        Ok(None)
    }
}
