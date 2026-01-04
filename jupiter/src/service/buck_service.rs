use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use bytes::Bytes;
use callisto::buck_session;
use callisto::entity_ext::generate_link;
use callisto::{mega_commit, mega_tree};
use chrono::{Duration, Utc};
use common::config::BuckConfig;
use common::errors::{BuckError, MegaError};
use sea_orm::TransactionTrait;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::service::cl_service::CLService;
use crate::service::git_service::GitService;
use crate::storage::base_storage::{BaseStorage, StorageConnector};
use crate::storage::buck_storage::{
    BuckStorage, FileRecord, session_status, upload_reason, upload_status,
};
use crate::storage::mono_storage::MonoStorage;

/// Buck upload service.
///
/// Handles buck upload operations including session management, manifest processing,
/// file uploads, and upload completion.
#[derive(Clone)]
pub struct BuckService {
    pub buck_storage: BuckStorage,
    pub mono_storage: MonoStorage,
    pub git_service: GitService,
    pub cl_service: CLService,
    pub upload_semaphore: Arc<Semaphore>,
    pub large_file_semaphore: Arc<Semaphore>,
    pub large_file_threshold: u64,
    pub session_timeout: i64,
    pub max_file_size: u64,
    pub max_files: u32,
    pub max_concurrent_uploads: u32,
}

/// Response for creating a buck upload session.
#[derive(Debug, Clone)]
pub struct SessionResponse {
    pub cl_link: String,
    pub expires_at: String,
    pub max_file_size: u64,
    pub max_files: u32,
    pub max_concurrent_uploads: u32,
}

/// Manifest file entry.
#[derive(Debug, Clone)]
pub struct ManifestFile {
    pub path: String,
    pub size: u64,
    pub hash: String,
}

/// Manifest payload.
#[derive(Debug, Clone)]
pub struct ManifestPayload {
    pub files: Vec<ManifestFile>,
    pub commit_message: Option<String>,
}

/// Manifest response containing file analysis results.
#[derive(Debug, Clone)]
pub struct ManifestResponse {
    pub total_files: u32,
    pub total_size: u64,
    pub files_to_upload: Vec<FileToUpload>,
    pub files_unchanged: u32,
    pub upload_size: u64,
}

#[derive(Debug, Clone)]
pub struct FileToUpload {
    pub path: String,
    pub reason: String,
}

/// Upload file response.
#[derive(Debug, Clone)]
pub struct FileUploadResponse {
    /// File path in repository
    pub file_path: String,
    pub uploaded_size: u64,
    pub verified: Option<bool>,
}

/// Complete upload payload.
#[derive(Debug, Clone)]
pub struct CompletePayload {
    pub commit_message: Option<String>,
}

/// Complete upload response.
#[derive(Debug, Clone)]
pub struct CompleteResponse {
    pub commit_id: String,
}

/// Commit artifacts passed from MonoApiService.
#[derive(Debug, Clone)]
pub struct CommitArtifacts {
    pub commit_id: String,
    pub tree_hash: String,
    pub new_tree_models: Vec<mega_tree::ActiveModel>,
    pub commit_model: mega_commit::ActiveModel,
}

impl BuckService {
    /// Create a new BuckService instance.
    pub fn new(
        base: BaseStorage,
        cl_service: CLService,
        upload_semaphore: Arc<Semaphore>,
        large_file_semaphore: Arc<Semaphore>,
        buck_config: BuckConfig,
        git_service: GitService,
    ) -> Result<Self, MegaError> {
        let buck_storage = BuckStorage { base: base.clone() };
        let mono_storage = MonoStorage { base: base.clone() };

        // Parse configuration values
        let max_file_size = buck_config.get_max_file_size_bytes().map_err(|e| {
            MegaError::Other(format!(
                "Failed to parse max_file_size '{}': {:#}. Please check your configuration. \
                Expected format: size with unit (e.g., '100MB', '2GB').",
                buck_config.max_file_size, e
            ))
        })?;

        let large_file_threshold = buck_config.get_large_file_threshold_bytes().map_err(|e| {
            MegaError::Other(format!(
                "Failed to parse large_file_threshold '{}': {:#}. Please check your configuration. \
                Expected format: size with unit (e.g., '1MB', '500KB').",
                buck_config.large_file_threshold, e
            ))
        })?;

        Self::validate_config(&buck_config, max_file_size, large_file_threshold)?;

        Ok(Self {
            buck_storage,
            mono_storage,
            git_service,
            cl_service,
            upload_semaphore,
            large_file_semaphore,
            large_file_threshold,
            session_timeout: buck_config.session_timeout as i64,
            max_file_size,
            max_files: buck_config.max_files,
            max_concurrent_uploads: buck_config.max_concurrent_uploads,
        })
    }

    /// Validates BuckConfig values for correctness and consistency.
    ///
    /// This function performs comprehensive validation including:
    /// - Platform limit checks (usize::MAX)
    /// - Logical consistency checks
    /// - Reasonableness checks
    fn validate_config(
        config: &BuckConfig,
        max_file_size: u64,
        large_file_threshold: u64,
    ) -> Result<(), MegaError> {
        // Validate max_file_size > 0
        if max_file_size == 0 {
            return Err(MegaError::Other(format!(
                "max_file_size must be greater than 0, got '{}' ({} bytes). \
                Please check your configuration.",
                config.max_file_size, max_file_size
            )));
        }

        // Validate max_file_size doesn't exceed platform limit
        let max_file_size_usize = usize::MAX as u64;
        if max_file_size > max_file_size_usize {
            return Err(MegaError::Other(format!(
                "max_file_size {} bytes (from '{}') exceeds platform limit {} bytes (usize::MAX). \
                This would cause runtime errors on this platform. \
                Please reduce max_file_size in configuration.",
                max_file_size, config.max_file_size, max_file_size_usize
            )));
        }

        // Validate large_file_threshold > 0
        if large_file_threshold == 0 {
            return Err(MegaError::Other(format!(
                "large_file_threshold must be greater than 0, got '{}' ({} bytes). \
                Please check your configuration.",
                config.large_file_threshold, large_file_threshold
            )));
        }

        // Validate large_file_threshold doesn't exceed platform limit
        if large_file_threshold > max_file_size_usize {
            return Err(MegaError::Other(format!(
                "large_file_threshold {} bytes (from '{}') exceeds platform limit {} bytes (usize::MAX). \
                This would cause runtime errors on this platform. \
                Please reduce large_file_threshold in configuration.",
                large_file_threshold, config.large_file_threshold, max_file_size_usize
            )));
        }

        // Validate large_file_threshold <= max_file_size (logical consistency)
        if large_file_threshold > max_file_size {
            return Err(MegaError::Other(format!(
                "large_file_threshold {} bytes (from '{}') exceeds max_file_size {} bytes (from '{}'). \
                This is logically inconsistent. large_file_threshold should be less than or equal to max_file_size. \
                Did you forget to update large_file_threshold when decreasing max_file_size? \
                Please adjust your configuration.",
                large_file_threshold,
                config.large_file_threshold,
                max_file_size,
                config.max_file_size
            )));
        }

        // Validate max_files > 0
        if config.max_files == 0 {
            return Err(MegaError::Other(format!(
                "max_files must be greater than 0, got {}. \
                Please check your configuration.",
                config.max_files
            )));
        }

        // Validate max_concurrent_uploads > 0
        if config.max_concurrent_uploads == 0 {
            return Err(MegaError::Other(format!(
                "max_concurrent_uploads must be greater than 0, got {}. \
                Please check your configuration.",
                config.max_concurrent_uploads
            )));
        }

        // Validate session_timeout > 0
        if config.session_timeout == 0 {
            return Err(MegaError::Other(format!(
                "session_timeout must be greater than 0, got {} seconds. \
                Please check your configuration.",
                config.session_timeout
            )));
        }

        Ok(())
    }

    /// Create a mock BuckService for tests.
    pub fn mock() -> Self {
        let base = BaseStorage::mock();
        let cl_service = CLService::mock();
        let upload_semaphore = Arc::new(Semaphore::new(10));
        let large_file_semaphore = Arc::new(Semaphore::new(5));
        let buck_config = BuckConfig::default();
        let git_service = GitService::mock();

        Self::new(
            base,
            cl_service,
            upload_semaphore,
            large_file_semaphore,
            buck_config,
            git_service,
        )
        .expect("mock BuckService should never fail")
    }

    /// Acquire upload permits (both global and large file if needed).
    ///
    /// This method encapsulates rate limiting logic, allowing Router to request
    /// permits without directly accessing semaphores.
    ///
    /// # Arguments
    /// * `file_size` - File size in bytes
    ///
    /// # Returns
    /// Tuple of (global_permit, optional_large_file_permit)
    pub fn try_acquire_upload_permits(
        &self,
        file_size: u64,
    ) -> Result<(OwnedSemaphorePermit, Option<OwnedSemaphorePermit>), MegaError> {
        // Acquire global permit
        let global_permit = self
            .upload_semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|_| MegaError::Buck(BuckError::RateLimitExceeded))?;

        // Acquire large file permit if needed
        let large_file_permit = if file_size >= self.large_file_threshold {
            Some(
                self.large_file_semaphore
                    .clone()
                    .try_acquire_owned()
                    .map_err(|_| MegaError::Buck(BuckError::RateLimitExceeded))?,
            )
        } else {
            None
        };

        Ok((global_permit, large_file_permit))
    }

    /// Get maximum file size for upload.
    pub fn max_file_size(&self) -> u64 {
        self.max_file_size
    }

    /// Create upload session and pre-create Draft CL.
    ///
    /// Generates a unique session ID, calculates expiration time, creates a Draft CL
    /// via CLService, and persists the session record. Returns session details including
    /// upload limits from configuration.
    ///
    /// # Arguments
    /// * `username` - User creating the session
    /// * `path` - Repository path
    /// * `from_hash` - Base commit hash (validated by upper layer)
    ///
    /// # Returns
    /// Returns `SessionResponse` with session details and upload limits
    pub async fn create_session(
        &self,
        username: &str,
        path: &str,
        from_hash: String,
    ) -> Result<SessionResponse, MegaError> {
        // Generate session_id
        let session_id = generate_link();

        // Calculate expiration time
        let expires_at = Utc::now() + Duration::seconds(self.session_timeout);

        // Pre-create Draft CL
        self.cl_service
            .create_draft_cl(path, &session_id, "Pending upload", &from_hash, username)
            .await?;

        // Create session record
        self.buck_storage
            .create_session(&session_id, username, path, &from_hash, expires_at)
            .await?;

        // Build response
        Ok(SessionResponse {
            cl_link: session_id,
            expires_at: expires_at.to_rfc3339(),
            max_file_size: self.max_file_size,
            max_files: self.max_files,
            max_concurrent_uploads: self.max_concurrent_uploads,
        })
    }

    /// Validate a manifest entry (path/hash).
    ///
    /// Performs comprehensive validation of file path and hash format.
    /// Validates path security (no absolute paths, path traversal, .git directories),
    /// and hash format.
    ///
    /// # Arguments
    /// * `path` - File path to validate
    /// * `hash` - Hash string to validate (must start with "sha1:" and be 40 hex chars)
    ///
    /// # Returns
    /// Returns `Ok(())` if validation passes, or `MegaError` with `BuckError` variant on failure
    pub fn validate_manifest_entry(&self, path: &str, hash: &str) -> Result<(), MegaError> {
        // Path checks
        if path.starts_with('/') {
            return Err(BuckError::ValidationError(format!(
                "Path must not start with '/': {}",
                path
            ))
            .into());
        }
        if path.contains('\\') {
            return Err(BuckError::ValidationError(format!(
                "Path must use '/' separator: {}",
                path
            ))
            .into());
        }
        // Windows absolute path like "C:/..."
        if path.len() >= 2 {
            let first_two = &path[..2];
            if first_two
                .chars()
                .next()
                .map(|c| c.is_ascii_alphabetic())
                .unwrap_or(false)
                && first_two.chars().nth(1) == Some(':')
            {
                return Err(BuckError::ValidationError(format!(
                    "Absolute path not allowed (Windows drive letter detected): {}",
                    path
                ))
                .into());
            }
        }
        if path.starts_with(".git/") || path.contains("/.git/") {
            return Err(
                BuckError::Forbidden(format!(".git directory not allowed: {}", path)).into(),
            );
        }
        if path.contains("..") {
            return Err(BuckError::ValidationError(format!(
                "Path traversal not allowed: {}",
                path
            ))
            .into());
        }

        if !hash.starts_with("sha1:") {
            return Err(BuckError::ValidationError(format!(
                "Hash must start with 'sha1:': {}",
                hash
            ))
            .into());
        }
        let hash_part = &hash[5..];
        if hash_part.len() != 40
            || !hash_part
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
        {
            return Err(BuckError::ValidationError(format!(
                "Invalid hash format (must be 40 lowercase hex chars): {}",
                hash
            ))
            .into());
        }

        // Path length & depth checks
        if path.len() > 4096 {
            return Err(BuckError::ValidationError(format!(
                "Path too long (max 4096 characters): {}",
                path
            ))
            .into());
        }
        let path_buf = PathBuf::from(path);
        let depth = path_buf.components().count();
        if depth > 100 {
            return Err(BuckError::ValidationError(format!(
                "Path nesting too deep (max 100 levels, got {}): {}",
                depth, path
            ))
            .into());
        }

        // Normalize path to ensure no bypass via "./" or duplicate separators
        let normalized = path_buf.components().collect::<PathBuf>();
        let normalized_str = normalized
            .to_str()
            .ok_or_else(|| {
                MegaError::Buck(BuckError::ValidationError(format!(
                    "Path contains invalid UTF-8 characters: {}",
                    path
                )))
            })?
            .replace('\\', "/");
        if normalized_str != path {
            return Err(BuckError::ValidationError(format!(
                "Path contains invalid components (normalized: {}): {}",
                normalized_str, path
            ))
            .into());
        }
        if path.contains("//")
            || path.contains("/./")
            || path.ends_with("/.")
            || path.starts_with("./")
            || path == "."
        {
            return Err(BuckError::ValidationError(format!(
                "Path contains invalid segments: {}",
                path
            ))
            .into());
        }

        Ok(())
    }

    /// Parse hash string by stripping the "sha1:" prefix if present.
    ///
    /// # Arguments
    /// * `hash` - Hash string that may or may not have "sha1:" prefix
    ///
    /// # Returns
    /// Returns the hash without the prefix
    pub fn parse_hash(&self, hash: &str) -> Result<String, MegaError> {
        let hash_str = hash.strip_prefix("sha1:").unwrap_or(hash);

        // Validate ObjectHash format
        if hash_str.len() != 40 {
            return Err(BuckError::ValidationError(format!(
                "Invalid ObjectHash hash length: expected 40, got {}",
                hash_str.len()
            ))
            .into());
        }

        Ok(hash_str.to_string())
    }

    /// Validate session ownership, expiration, and status.
    ///
    /// Checks that the session exists, belongs to the specified user, has not expired,
    /// and is in one of the allowed statuses.
    ///
    /// # Arguments
    /// * `session_id` - Session ID to validate
    /// * `username` - Expected session owner
    /// * `allowed_statuses` - List of allowed session statuses
    ///
    /// # Returns
    /// Returns the session model if validation passes, or `MegaError` on failure
    pub async fn validate_session(
        &self,
        session_id: &str,
        username: &str,
        allowed_statuses: &[&str],
    ) -> Result<buck_session::Model, MegaError> {
        let session = self
            .buck_storage
            .get_session(session_id)
            .await?
            .ok_or_else(|| MegaError::Buck(BuckError::SessionNotFound(session_id.to_string())))?;

        if session.user_id != username {
            return Err(BuckError::Forbidden("Session belongs to another user".to_string()).into());
        }
        if session.expires_at < Utc::now().naive_utc() {
            return Err(BuckError::SessionExpired.into());
        }
        if !allowed_statuses.contains(&session.status.as_str()) {
            return Err(BuckError::InvalidSessionStatus {
                expected: allowed_statuses.join(", "),
                actual: session.status.to_string(),
            }
            .into());
        }

        Ok(session)
    }

    /// Process manifest and determine which files need to be uploaded.
    ///
    /// Validates the manifest, compares files with existing versions, and creates
    /// file records in the database. Updates session status to MANIFEST_UPLOADED.
    ///
    /// # Arguments
    /// * `username` - User processing the manifest
    /// * `session_id` - Session ID
    /// * `payload` - Manifest payload containing files to process
    /// * `existing_file_hashes` - Map of existing file paths to their blob hashes
    ///
    /// # Returns
    /// Returns `ManifestResponse` with analysis results
    pub async fn process_manifest(
        &self,
        username: &str,
        cl_link: &str,
        payload: ManifestPayload,
        existing_file_hashes: HashMap<PathBuf, String>,
    ) -> Result<ManifestResponse, MegaError> {
        // Basic validation
        if payload.files.is_empty() {
            return Err(BuckError::ValidationError("Empty file list".to_string()).into());
        }
        if payload.files.len() > self.max_files as usize {
            return Err(BuckError::ValidationError(format!(
                "File count {} exceeds limit {}",
                payload.files.len(),
                self.max_files
            ))
            .into());
        }

        // Validate session status (owner/status/not expired)
        self.validate_session(cl_link, username, &[session_status::CREATED])
            .await?;

        // Validate and compare each file
        let mut seen_paths = HashSet::new();
        let mut files_to_upload = Vec::new();
        let mut files_unchanged = 0u32;
        let mut upload_size = 0u64;
        let mut total_size = 0u64;
        let mut file_records = Vec::new();

        for file in &payload.files {
            self.validate_manifest_entry(&file.path, &file.hash)?;
            if !seen_paths.insert(&file.path) {
                return Err(BuckError::ValidationError(format!(
                    "Duplicate file path in manifest: {}",
                    file.path
                ))
                .into());
            }

            // Check for overflow when accumulating total_size
            total_size = total_size.checked_add(file.size).ok_or_else(|| {
                BuckError::ValidationError(format!(
                    "Total file size exceeds maximum (overflow detected). File: {}",
                    file.path
                ))
            })?;

            let path_buf = PathBuf::from(&file.path);
            let new_hash = self.parse_hash(&file.hash)?;

            let (status, reason, existing_blob_id) = match existing_file_hashes.get(&path_buf) {
                None => {
                    files_to_upload.push(FileToUpload {
                        path: file.path.clone(),
                        reason: upload_reason::NEW.to_string(),
                    });
                    // Check for overflow when accumulating upload_size
                    upload_size = upload_size.checked_add(file.size).ok_or_else(|| {
                        BuckError::ValidationError(format!(
                            "Total upload size exceeds maximum (overflow detected). File: {}",
                            file.path
                        ))
                    })?;
                    (
                        upload_status::PENDING.to_string(),
                        Some(upload_reason::NEW.to_string()),
                        None,
                    )
                }
                Some(old_hash) => {
                    // Normalize old_hash for comparison
                    let normalized_old_hash = self.parse_hash(old_hash)?;
                    if normalized_old_hash != new_hash {
                        files_to_upload.push(FileToUpload {
                            path: file.path.clone(),
                            reason: upload_reason::MODIFIED.to_string(),
                        });
                        // Check for overflow when accumulating upload_size
                        upload_size = upload_size.checked_add(file.size).ok_or_else(|| {
                            BuckError::ValidationError(format!(
                                "Total upload size exceeds maximum (overflow detected). File: {}",
                                file.path
                            ))
                        })?;
                        (
                            upload_status::PENDING.to_string(),
                            Some(upload_reason::MODIFIED.to_string()),
                            None,
                        )
                    } else {
                        files_unchanged += 1;
                        (
                            upload_status::SKIPPED.to_string(),
                            None,
                            Some(format!("sha1:{}", normalized_old_hash)),
                        )
                    }
                }
            };

            file_records.push(FileRecord {
                file_path: file.path.clone(),
                file_size: file.size as i64,
                file_hash: file.hash.clone(),
                file_mode: Some("100644".to_string()), // Always use default mode
                upload_status: status,
                upload_reason: reason,
                blob_id: existing_blob_id,
            });
        }

        // Batch insert file records (idempotent via ON CONFLICT DO NOTHING)
        self.buck_storage
            .batch_insert_files(cl_link, file_records)
            .await?;

        // Update session status
        self.buck_storage
            .update_session_status_with_pool(
                cl_link,
                session_status::MANIFEST_UPLOADED,
                payload.commit_message.as_deref(),
            )
            .await?;

        Ok(ManifestResponse {
            total_files: payload.files.len() as u32,
            total_size,
            files_to_upload,
            files_unchanged,
            upload_size,
        })
    }

    /// Upload a file and persist its content to storage.
    ///
    /// Validates the file, writes blob content to storage, verifies hash if provided,
    /// and updates file status in the database.
    ///
    /// # Arguments
    /// * `username` - User uploading the file
    /// * `cl_link` - CL link (8-character alphanumeric identifier)
    /// * `file_path` - Path of the file to upload
    /// * `file_size` - Expected file size in bytes
    /// * `file_hash` - Optional expected hash for verification
    /// * `file_content` - File content bytes
    ///
    /// # Returns
    /// Returns `FileUploadResponse` with upload details
    pub async fn upload_file(
        &self,
        username: &str,
        cl_link: &str,
        file_path: &str,
        file_size: u64,
        file_hash: Option<&str>,
        file_content: Bytes,
    ) -> Result<FileUploadResponse, MegaError> {
        // Validate session status (manifest_uploaded or uploading)
        self.validate_session(
            cl_link,
            username,
            &[session_status::MANIFEST_UPLOADED, session_status::UPLOADING],
        )
        .await?;

        // Get pending record
        let pending = self
            .buck_storage
            .get_pending_file(cl_link, file_path)
            .await?
            .ok_or_else(|| MegaError::Buck(BuckError::FileNotInManifest(file_path.to_string())))?;

        // Validate size (prevent exceeding limit)
        if file_size > self.max_file_size {
            return Err(BuckError::FileSizeExceedsLimit(file_size, self.max_file_size).into());
        }

        // Validate length (consistent with header declaration)
        if file_content.len() as u64 != file_size {
            return Err(BuckError::ValidationError(format!(
                "Size mismatch: header says {}, got {}",
                file_size,
                file_content.len()
            ))
            .into());
        }

        // Write blob to storage
        let blob_hash = self.git_service.save_object_from_raw(file_content).await?;

        // Optional hash verification
        let verified = if let Some(expected_hash) = file_hash {
            let normalized_expected = self.parse_hash(expected_hash)?;
            if normalized_expected != blob_hash {
                return Err(BuckError::HashMismatch {
                    expected: normalized_expected,
                    actual: blob_hash,
                }
                .into());
            }
            Some(true)
        } else {
            // Normalize the hash from manifest (may have "sha1:" prefix)
            let normalized_record = self.parse_hash(&pending.file_hash)?;
            if normalized_record != blob_hash {
                return Err(BuckError::HashMismatch {
                    expected: normalized_record,
                    actual: blob_hash,
                }
                .into());
            }
            Some(true)
        };
        let rows = self
            .buck_storage
            .mark_file_uploaded(cl_link, file_path, &blob_hash)
            .await?;

        if rows == 0 {
            return Err(BuckError::FileAlreadyUploaded(file_path.to_string()).into());
        }

        // Atomically switch from manifest_uploaded to uploading if needed
        // Use atomic compare-and-swap to prevent race conditions when multiple
        // concurrent uploads try to update the session status simultaneously.
        self.buck_storage
            .update_session_status_if_current_with_pool(
                cl_link,
                session_status::MANIFEST_UPLOADED,
                session_status::UPLOADING,
                None,
            )
            .await?;

        Ok(FileUploadResponse {
            file_path: file_path.to_string(),
            uploaded_size: file_size,
            verified,
        })
    }

    /// Complete the upload process and persist commit artifacts.
    ///
    /// Validates all files are uploaded, then persists commit artifacts (trees, commit,
    /// CL refs, and CL record) within a database transaction. Updates session status to COMPLETED.
    ///
    /// # Arguments
    /// * `username` - User completing the upload
    /// * `cl_link` - CL link (8-character alphanumeric identifier)
    /// * `payload` - Complete payload containing an optional commit message
    /// * `commit_artifacts` - Optional commit artifacts from MonoApiService (Git build in ceres)
    ///
    /// # Returns
    /// Returns `CompleteResponse` with commit ID
    pub async fn complete_upload(
        &self,
        username: &str,
        cl_link: &str,
        payload: CompletePayload,
        commit_artifacts: Option<CommitArtifacts>,
    ) -> Result<CompleteResponse, MegaError> {
        // Validate session status
        let session = self
            .validate_session(
                cl_link,
                username,
                &[session_status::MANIFEST_UPLOADED, session_status::UPLOADING],
            )
            .await?;

        // Ensure no pending files
        let pending = self.buck_storage.count_pending_files(cl_link).await?;
        if pending > 0 {
            return Err(BuckError::FilesNotFullyUploaded {
                missing_count: pending as u32,
            }
            .into());
        }

        // Get all files and verify blob_id exists
        let all_files = self.buck_storage.get_all_files(cl_link).await?;
        for file in &all_files {
            if file.blob_id.is_none() {
                return Err(BuckError::FilesNotFullyUploaded {
                    missing_count: 1, // At least one file is missing
                }
                .into());
            }
        }

        // Extract commit information (if present)
        let commit_id = if let Some(artifacts) = &commit_artifacts {
            artifacts.commit_id.clone()
        } else {
            session.from_hash.clone().unwrap_or_default()
        };

        // Persist within transaction
        let db = self.mono_storage.get_connection();
        let txn = db.begin().await?;

        if let Some(artifacts) = commit_artifacts {
            // Save trees
            self.mono_storage
                .save_trees_batch(&txn, artifacts.new_tree_models)
                .await?;

            // Save commit
            self.mono_storage
                .save_commit_in_txn(&txn, artifacts.commit_model)
                .await?;

            // Update/create CL ref
            let cl_ref_name = format!("refs/cl/{}", cl_link);
            self.mono_storage
                .save_or_update_cl_ref_in_txn(
                    &txn,
                    &session.repo_path,
                    &cl_ref_name,
                    &artifacts.commit_id,
                    &artifacts.tree_hash,
                )
                .await?;

            // Update CL record
            self.mono_storage
                .get_and_update_cl_in_txn(
                    &txn,
                    cl_link,
                    session.from_hash.as_deref().unwrap_or_default(),
                    &artifacts.commit_id,
                    payload
                        .commit_message
                        .as_deref()
                        .unwrap_or("Upload via buck push"),
                )
                .await?;
        }

        // Update session status to completed
        self.buck_storage
            .update_session_status(
                &txn,
                cl_link,
                session_status::COMPLETED,
                payload.commit_message.as_deref(),
            )
            .await?;

        txn.commit().await?;

        // TODO: Buck Upload completion flow - remaining steps (not implemented):
        // 1. Output CL creation and diff logs (need to calculate file diffs)
        // 2. Notify change-detector (need to implement change-detector client)
        // 3. Buck2 build flow (need to integrate bellatrix and dependency analysis):
        //    - Analyze affected targets (based on BUCK dependency graph)
        //    - Return build target list (affected_targets)
        //    - Start build tasks (only build affected_targets)
        //    - Track build progress and results
        //    - Push build progress and result logs

        Ok(CompleteResponse { commit_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::base_storage::{BaseStorage, StorageConnector};
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    fn create_test_service() -> BuckService {
        let base = BaseStorage::mock();
        let upload_semaphore = Arc::new(Semaphore::new(10));
        let large_file_semaphore = Arc::new(Semaphore::new(5));
        let buck_config = common::config::BuckConfig::default();
        BuckService::new(
            base,
            CLService::new(BaseStorage::mock()),
            upload_semaphore,
            large_file_semaphore,
            buck_config,
            GitService::mock(),
        )
        .expect("Failed to create test BuckService")
    }

    // ============================================================================
    // validate_manifest_entry tests
    // ============================================================================

    #[test]
    fn test_validate_rejects_absolute_path() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            "/absolute/path.txt",
            "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3",
        );
        assert!(result.is_err(), "Absolute path should be rejected");
    }

    #[test]
    fn test_validate_rejects_backslash() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            "path\\to\\file.txt",
            "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3",
        );
        assert!(result.is_err(), "Backslash separator should be rejected");
    }

    #[test]
    fn test_validate_rejects_windows_absolute_path() {
        let service = create_test_service();
        let windows_paths = vec![
            "C:/Windows/System32/config/sam",
            "C:\\Windows\\System32\\config\\sam",
            "D:/Users/test.txt",
            "Z:/path/to/file",
            "A:/root",
        ];

        for path in windows_paths {
            let result = service
                .validate_manifest_entry(path, "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
            assert!(
                result.is_err(),
                "Windows absolute path should be rejected: {}",
                path
            );
        }
    }

    #[test]
    fn test_validate_accepts_relative_path() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            "relative/path.txt",
            "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3",
        );
        assert!(result.is_ok(), "Relative path should be valid");
    }

    #[test]
    fn test_validate_rejects_git_directory_at_root() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            ".git/config",
            "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3",
        );
        assert!(result.is_err(), ".git directory at root should be rejected");
    }

    #[test]
    fn test_validate_rejects_git_directory_nested() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            "submodule/.git/config",
            "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3",
        );
        assert!(result.is_err(), "Nested .git directory should be rejected");
    }

    #[test]
    fn test_validate_rejects_git_directory_deeply_nested() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            "a/b/c/.git/objects/pack",
            "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3",
        );
        assert!(result.is_err(), "Deeply nested .git should be rejected");
    }

    #[test]
    fn test_validate_allows_gitignore_file() {
        // ".gitignore" is NOT ".git/" - should be allowed
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            ".gitignore",
            "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3",
        );
        assert!(result.is_ok(), ".gitignore file should be allowed");
    }

    #[test]
    fn test_validate_allows_gitkeep_file() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            "empty_dir/.gitkeep",
            "sha1:e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
        );
        assert!(result.is_ok(), ".gitkeep file should be allowed");
    }

    #[test]
    fn test_validate_rejects_path_traversal_simple() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            "../etc/passwd",
            "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3",
        );
        assert!(result.is_err(), "Path traversal should be rejected");
    }

    #[test]
    fn test_validate_rejects_path_traversal_in_middle() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            "a/b/../../../etc/passwd",
            "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3",
        );
        assert!(
            result.is_err(),
            "Path traversal in middle should be rejected"
        );
    }

    #[test]
    fn test_validate_rejects_hash_without_prefix() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            "file.txt",
            "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3", // Missing sha1:
        );
        assert!(
            result.is_err(),
            "Hash without sha1: prefix should be rejected"
        );
    }

    #[test]
    fn test_validate_rejects_hash_wrong_length() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            "file.txt",
            "sha1:abc123", // Too short
        );
        assert!(result.is_err(), "Hash with wrong length should be rejected");
    }

    #[test]
    fn test_validate_rejects_hash_uppercase() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            "file.txt",
            "sha1:A94A8FE5CCB19BA61C4C0873D391E987982FBBD3", // Uppercase
        );
        assert!(result.is_err(), "Uppercase hash should be rejected");
    }

    #[test]
    fn test_validate_rejects_hash_non_hex() {
        let service = create_test_service();
        let result = service.validate_manifest_entry(
            "file.txt",
            "sha1:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz", // Invalid chars
        );
        assert!(result.is_err(), "Non-hex characters should be rejected");
    }

    #[test]
    fn test_validate_accepts_valid_hash() {
        let service = create_test_service();
        let result = service
            .validate_manifest_entry("file.txt", "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
        assert!(result.is_ok(), "Valid hash should be accepted");
    }

    #[test]
    fn test_validate_rejects_path_too_long() {
        // Create a path longer than 4096 characters
        let long_path = "a/".repeat(2500) + "file.txt";
        assert!(long_path.len() > 4096, "Test path should exceed 4096 chars");

        let service = create_test_service();
        let result = service
            .validate_manifest_entry(&long_path, "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
        assert!(
            result.is_err(),
            "Path longer than 4096 characters should be rejected"
        );
    }

    #[test]
    fn test_validate_rejects_nesting_too_deep() {
        // Create a path with more than 100 levels
        let deep_path = "level".to_string() + &"/level".repeat(149) + "/file.txt";
        let path = std::path::PathBuf::from(&deep_path);
        let depth = path.components().count();
        assert!(
            depth > 100,
            "Test path should have more than 100 levels (got {})",
            depth
        );

        let service = create_test_service();
        let result = service
            .validate_manifest_entry(&deep_path, "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
        assert!(
            result.is_err(),
            "Path with nesting deeper than 100 levels should be rejected"
        );
    }

    #[test]
    fn test_validate_accepts_path_at_limit() {
        // Path exactly at 4096 characters should be accepted
        let limit_path = "a".repeat(4092) + ".txt"; // 4092 + 4 = 4096
        assert_eq!(limit_path.len(), 4096);

        let service = create_test_service();
        let result = service
            .validate_manifest_entry(&limit_path, "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
        assert!(
            result.is_ok(),
            "Path at exactly 4096 characters should be accepted"
        );
    }

    #[test]
    fn test_validate_accepts_nesting_at_limit() {
        // Path with exactly 100 levels should be accepted
        let limit_path = "a/".repeat(99) + "file.txt"; // 99 dirs + 1 file = 100 components
        let path = std::path::PathBuf::from(&limit_path);
        let depth = path.components().count();
        assert_eq!(depth, 100, "Test path should have exactly 100 levels");

        let service = create_test_service();
        let result = service
            .validate_manifest_entry(&limit_path, "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
        assert!(
            result.is_ok(),
            "Path with exactly 100 levels should be accepted"
        );
    }

    #[test]
    fn test_validate_rejects_invalid_path_segments() {
        let service = create_test_service();
        let invalid_paths = vec![
            "a//b",  // Double slash
            "a/./b", // Current directory
            "a/.",   // Ends with dot
            "./a",   // Starts with dot-slash
            ".",     // Just dot
        ];

        for path in invalid_paths {
            let result = service
                .validate_manifest_entry(path, "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
            assert!(
                result.is_err(),
                "Invalid path segment should be rejected: {}",
                path
            );
        }
    }

    // ============================================================================
    // parse_hash tests
    // ============================================================================

    #[test]
    fn test_parse_hash_with_prefix() {
        let service = create_test_service();
        let result = service.parse_hash("sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
    }

    #[test]
    fn test_parse_hash_without_prefix() {
        let service = create_test_service();
        let result = service.parse_hash("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
    }

    #[test]
    fn test_parse_hash_rejects_wrong_length() {
        let service = create_test_service();
        let result = service.parse_hash("sha1:abc123");
        assert!(result.is_err(), "Should reject hash with wrong length");
    }
}
