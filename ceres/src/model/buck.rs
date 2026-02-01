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
    /// CL link (8-character alphanumeric identifier, same as session_id)
    pub cl_link: String,
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
    /// File hash in "algorithm:HEXSTRING" format (case-insensitive, normalized to lowercase)
    ///
    /// **Multi-hash support**: Accepts both SHA-1 and SHA-256:
    /// - SHA-1: "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709" (40 hex chars)
    /// - SHA-256: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" (64 hex chars)
    pub hash: String,
}

/// Parse and validate ObjectHash in "algorithm:HEXSTRING" format
///
/// **Multi-hash support**: Accepts both SHA-1 (40 hex chars) and SHA-256 (64 hex chars).
/// This is a shared helper function used by both ManifestFile and FileChange.
/// It normalizes the hash to lowercase for consistency with Git conventions.
///
/// # Arguments
/// * `input` - The hash string in "algorithm:HEXSTRING" format
/// * `field_name` - The name of the field (for error messages)
///
/// # Supported Algorithms
/// - `sha1`: 40-character hex string
/// - `sha256`: 64-character hex string
///
/// # Returns
/// The parsed ObjectHash or an error if format is invalid
pub fn parse_object_hash(
    input: &str,
    field_name: &str,
) -> Result<git_internal::hash::ObjectHash, common::errors::MegaError> {
    use std::str::FromStr;

    use common::errors::MegaError;
    use git_internal::hash::ObjectHash;

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
        "sha1" => {
            // Validate SHA-1 hash length (40 hex chars)
            if hash_hex.len() != 40 {
                return Err(MegaError::Other(format!(
                    "Invalid SHA-1 hash length in {}: expected 40 chars, got {}",
                    field_name,
                    hash_hex.len()
                )));
            }
            ObjectHash::from_str(&hash_hex).map_err(|e| {
                MegaError::Other(format!(
                    "Invalid ObjectHash in {}: '{}', error: {}",
                    field_name, hash_hex, e
                ))
            })
        }
        "sha256" => {
            // Validate SHA-256 hash length (64 hex chars)
            if hash_hex.len() != 64 {
                return Err(MegaError::Other(format!(
                    "Invalid SHA-256 hash length in {}: expected 64 chars, got {}",
                    field_name,
                    hash_hex.len()
                )));
            }
            ObjectHash::from_str(&hash_hex).map_err(|e| {
                MegaError::Other(format!(
                    "Invalid ObjectHash in {}: '{}', error: {}",
                    field_name, hash_hex, e
                ))
            })
        }
        other => Err(MegaError::Other(format!(
            "Unsupported hash algorithm in {}: '{}', supported: 'sha1', 'sha256'",
            field_name, other
        ))),
    }
}

/// Backward-compatible alias for parse_object_hash
///
/// **Deprecated**: Use `parse_object_hash` instead for multi-hash support.
#[deprecated(since = "0.2.0", note = "Use parse_object_hash for multi-hash support")]
pub fn parse_sha1_hash(
    input: &str,
    field_name: &str,
) -> Result<git_internal::hash::ObjectHash, common::errors::MegaError> {
    parse_object_hash(input, field_name)
}

impl ManifestFile {
    /// Parse and validate hash format
    ///
    /// **Multi-hash support**: Accepts both SHA-1 and SHA-256 formats.
    /// Expects format: "algorithm:HEXSTRING" (case-insensitive, normalized to lowercase)
    /// Returns the parsed ObjectHash or an error if format is invalid
    pub fn parse_hash(&self) -> Result<git_internal::hash::ObjectHash, common::errors::MegaError> {
        parse_object_hash(&self.hash, "hash field")
    }
}

/// Default file mode used when mode is not provided by client
pub const DEFAULT_MODE: &str = "100644";

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
    /// File path in repository (relative to repo root; not a local filesystem path)
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
/// Note:
/// - Does not include build status (build is triggered asynchronously).
/// - When there are no file changes, no new commit is created. In that case
///   `commit_id` may be empty or equal to the session's base commit hash
///   (`from_hash`, if provided). Clients must tolerate an empty `commit_id`
///   for the "no-change" completion path.
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
    /// ObjectHash of the blob in "algorithm:HEXSTRING" format (case-insensitive, normalized to lowercase)
    ///
    /// **Multi-hash support**: Accepts both SHA-1 and SHA-256:
    /// - SHA-1: "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709" (40 hex chars)
    /// - SHA-256: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" (64 hex chars)
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
    /// **Multi-hash support**: Accepts both SHA-1 and SHA-256 formats.
    /// Expects format: "algorithm:HEXSTRING" (case-insensitive, normalized to lowercase)
    /// Returns the parsed ObjectHash or an error if format is invalid
    #[allow(deprecated)]
    pub fn parse_blob_hash(
        &self,
    ) -> Result<git_internal::hash::ObjectHash, common::errors::MegaError> {
        parse_object_hash(&self.blob_id, "blob_id")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_object_hash_sha1_valid() {
        let result = parse_object_hash("sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3", "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_object_hash_sha256_valid() {
        let result = parse_object_hash(
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "test",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_object_hash_sha1_wrong_length() {
        let result = parse_object_hash("sha1:abc123", "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected 40"));
    }

    #[test]
    fn test_parse_object_hash_sha256_wrong_length() {
        let result = parse_object_hash("sha256:abc123", "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected 64"));
    }

    #[test]
    fn test_parse_object_hash_unsupported_algorithm() {
        let result = parse_object_hash("md5:abc123", "test");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported hash algorithm")
        );
    }

    #[test]
    fn test_parse_object_hash_missing_colon() {
        let result = parse_object_hash("sha1abc123", "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("algorithm:hex"));
    }

    #[test]
    fn test_parse_object_hash_case_insensitive() {
        // Algorithm should be case-insensitive
        let result = parse_object_hash("SHA1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3", "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_manifest_file_parse_hash_sha256() {
        let file = ManifestFile {
            path: "test.txt".to_string(),
            size: 100,
            hash: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                .to_string(),
        };
        assert!(file.parse_hash().is_ok());
    }

    #[test]
    fn test_file_change_parse_blob_hash_sha256() {
        let change = FileChange::new(
            "test.txt".to_string(),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            "100644".to_string(),
        );
        assert!(change.parse_blob_hash().is_ok());
    }
}
