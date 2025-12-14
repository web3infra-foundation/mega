//! Buck upload API data models
//!
//! This module contains request and response structures for the Buck upload API.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request payload for creating an upload session
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSessionPayload {
    /// Repository path, e.g. "/project/mega"
    pub path: String,
}

/// Response for session creation
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionResponse {
    /// Unique session identifier (8 characters)
    pub session_id: String,
    /// Session expiration time (RFC3339 format)
    pub expires_at: String,
    /// Maximum file size in bytes
    pub max_file_size: u64,
    /// Maximum number of files per session
    pub max_files: u32,
    /// Recommended concurrent upload count
    pub max_concurrent_uploads: u32,
}

/// Request payload for uploading file manifest
#[derive(Debug, Deserialize, ToSchema)]
pub struct ManifestPayload {
    /// List of files to upload
    pub files: Vec<ManifestFile>,
    /// Optional commit message
    pub commit_message: Option<String>,
}

/// File entry in the manifest
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ManifestFile {
    /// Relative file path (must not start with '/')
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// File hash in "sha1:HEXSTRING" format (case-insensitive, normalized to lowercase)
    /// Example: "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709"
    pub hash: String,
    /// File mode (100644, 100755, or 120000)
    /// Defaults to 100644 (regular file) if not specified
    #[serde(default = "default_mode")]
    pub mode: String,
}

/// Parse and validate SHA1 hash in "sha1:HEXSTRING" format
///
/// This is a shared helper function used by both ManifestFile and FileChange.
/// It normalizes the hash to lowercase for consistency with Git conventions.
///
/// # Arguments
/// * `input` - The hash string in "sha1:HEXSTRING" format
/// * `field_name` - The name of the field (for error messages)
///
/// # Returns
/// The parsed SHA1 hash or an error if format is invalid
pub fn parse_sha1_hash(
    input: &str,
    field_name: &str,
) -> Result<git_internal::hash::SHA1, common::errors::MegaError> {
    use common::errors::MegaError;
    use git_internal::hash::SHA1;
    use std::str::FromStr;

    let parts: Vec<&str> = input.splitn(2, ':').collect();

    if parts.len() != 2 {
        return Err(MegaError::Other(format!(
            "Invalid {} format, expected 'algorithm:hex', got: {}",
            field_name, input
        )));
    }

    let algorithm = parts[0].to_lowercase();
    let hash_hex = parts[1].to_lowercase(); // Normalize to lowercase (Git convention)

    match algorithm.as_str() {
        "sha1" => SHA1::from_str(&hash_hex).map_err(|e| {
            MegaError::Other(format!(
                "Invalid SHA1 hash in {}: '{}', error: {}",
                field_name, hash_hex, e
            ))
        }),
        other => Err(MegaError::Other(format!(
            "Unsupported hash algorithm in {}: '{}', only 'sha1' is supported",
            field_name, other
        ))),
    }
}

impl ManifestFile {
    /// Parse and validate hash format
    ///
    /// Expects format: "sha1:HEXSTRING" (case-insensitive, normalized to lowercase)
    /// Returns the parsed SHA1 hash or an error if format is invalid
    pub fn parse_hash(&self) -> Result<git_internal::hash::SHA1, common::errors::MegaError> {
        parse_sha1_hash(&self.hash, "hash field")
    }
}

const DEFAULT_MODE: &str = "100644";

fn default_mode() -> String {
    DEFAULT_MODE.to_string()
}

/// Response for manifest upload
#[derive(Debug, Serialize, ToSchema)]
pub struct ManifestResponse {
    /// Total number of files in manifest
    pub total_files: u32,
    /// Total size of all files in bytes
    pub total_size: u64,
    /// List of files that need to be uploaded
    pub files_to_upload: Vec<FileToUpload>,
    /// Number of unchanged files (skipped)
    pub files_unchanged: u32,
    /// Total size of files to upload in bytes
    pub upload_size: u64,
}

/// File that needs to be uploaded
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileToUpload {
    /// File path
    pub path: String,
    /// Upload reason: "new" or "modified"
    pub reason: String,
}

/// Response for file upload
#[derive(Debug, Serialize, ToSchema)]
pub struct FileUploadResponse {
    /// File path in repository (relative to repository root, not local filesystem path)
    pub file_path: String,
    /// Uploaded file size in bytes
    pub uploaded_size: u64,
    /// Whether hash verification passed (if hash was provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
}

/// Request payload for completing upload
#[derive(Debug, Deserialize, ToSchema)]
pub struct CompletePayload {
    /// Optional commit message (overrides manifest message)
    pub commit_message: Option<String>,
}

/// Response for upload completion
///
/// Note: Does not include build status. Build is triggered asynchronously.
#[derive(Debug, Serialize, ToSchema)]
pub struct CompleteResponse {
    /// Change List ID
    pub cl_id: i64,
    /// CL link (same as session_id)
    pub cl_link: String,
    /// Created commit hash
    pub commit_id: String,
    /// Total number of files in the commit
    pub files_count: u32,
    /// CL creation time (RFC3339 format)
    pub created_at: String,
}

/// Represents a single file change for batch commit building
///
/// This structure is used by `BuckCommitBuilder` to batch multiple file
/// changes into a single Git commit.
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Relative file path within the repository (e.g., "src/main.rs")
    pub path: String,
    /// SHA1 hash of the blob in "sha1:HEXSTRING" format (case-insensitive, normalized to lowercase)
    /// Already saved in raw_blob table
    /// Example: "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709"
    pub blob_id: String,
    /// File mode: "100644" (normal), "100755" (executable), "120000" (symlink)
    pub mode: String,
}

impl FileChange {
    pub fn new(path: String, blob_id: String, mode: String) -> Self {
        Self {
            path,
            blob_id,
            mode,
        }
    }

    /// Parse and validate blob hash format
    ///
    /// Expects format: "sha1:HEXSTRING" (case-insensitive, normalized to lowercase)
    /// Returns the parsed SHA1 hash or an error if format is invalid
    pub fn parse_blob_hash(&self) -> Result<git_internal::hash::SHA1, common::errors::MegaError> {
        parse_sha1_hash(&self.blob_id, "blob_id")
    }

    /// Parse mode string to TreeItemMode
    ///
    /// Validates and converts Git file mode strings to internal TreeItemMode enum.
    /// Logs a warning and defaults to normal blob (100644) for invalid modes.
    pub fn tree_item_mode(&self) -> git_internal::internal::object::tree::TreeItemMode {
        use git_internal::internal::object::tree::TreeItemMode;
        match self.mode.as_str() {
            "100644" => TreeItemMode::Blob,
            "100755" => TreeItemMode::BlobExecutable,
            "120000" => TreeItemMode::Link,
            _ => {
                tracing::warn!(
                    "Invalid file mode '{}' for path '{}', defaulting to 100644",
                    self.mode,
                    self.path
                );
                TreeItemMode::Blob
            }
        }
    }
}
