//! Buck upload API router
//!
//! This module provides HTTP routes for the Buck upload batch API.

use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, Request, StatusCode},
};
use chrono::{Duration, Utc};
use common::model::CommonResult;
use tokio::sync::TryAcquireError;
use utoipa_axum::{router::OpenApiRouter, routes};

use ceres::api_service::blob_ops;
use ceres::model::buck::*;
use jupiter::storage::base_storage::StorageConnector;
use jupiter::storage::buck_storage::{FileRecord, session_status, upload_reason, upload_status};
use jupiter::utils::converter::IntoMegaModel;
use sea_orm::entity::IntoActiveModel;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::api::{MonoApiServiceState, error::ApiError, oauth::model::LoginUser};
use crate::server::http_server::BUCK_TAG;

// Fault injection constants (compiled only in test mode)
#[cfg(test)]
pub mod injection {
    pub const FAIL_TREE_SAVE: &str = "BUCK_FAIL_TREE_SAVE";
    pub const FAIL_COMMIT_SAVE: &str = "BUCK_FAIL_COMMIT_SAVE";
    pub const FAIL_REF_UPDATE: &str = "BUCK_FAIL_REF_UPDATE";
    pub const FAIL_CL_UPDATE: &str = "BUCK_FAIL_CL_UPDATE";
}

/// Macro to inject a failure point for testing
/// Returns early with an error if the environment variable is set
#[cfg(test)]
macro_rules! inject_fail {
    ($env_var:expr, $msg:expr, $error_msg:expr) => {
        if std::env::var($env_var).is_ok() {
            tracing::warn!($msg);
            return Err(ApiError::internal(anyhow::anyhow!($error_msg)));
        }
    };
}

#[cfg(not(test))]
macro_rules! inject_fail {
    ($env_var:expr, $msg:expr, $error_msg:expr) => {};
}

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/buck",
        OpenApiRouter::new()
            .routes(routes!(create_session))
            .routes(routes!(upload_manifest))
            .routes(routes!(upload_file))
            .routes(routes!(complete_upload)),
    )
}

/// Create upload session
///
/// Creates a new upload session and pre-creates a Draft CL.
#[utoipa::path(
    post,
    path = "/session/start",
    request_body = CreateSessionPayload,
    responses(
        (status = 200, body = CommonResult<SessionResponse>),
        (status = 400, description = "Invalid request parameters"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Path not found"),
    ),
    tag = BUCK_TAG
)]
async fn create_session(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(payload): Json<CreateSessionPayload>,
) -> Result<Json<CommonResult<SessionResponse>>, ApiError> {
    // Validate path
    let path = payload.path.trim();
    if path.is_empty() {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Path cannot be empty"
        )));
    }

    // Get current commit hash (verify path exists)
    let refs = state
        .storage
        .mono_storage()
        .get_main_ref(path)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| {
            ApiError::with_status(
                StatusCode::NOT_FOUND,
                anyhow::anyhow!("Path not found: {}", path),
            )
        })?;
    let from_hash = refs.ref_commit_hash;

    // Generate session_id
    let session_id = callisto::entity_ext::generate_link();

    // Get config
    let config = state.storage.config();
    let buck_config = config.buck.as_ref();
    let timeout = buck_config.map(|b| b.session_timeout).unwrap_or(3600);
    let expires_at = Utc::now() + Duration::seconds(timeout as i64);

    // Pre-create Draft CL
    state
        .cl_stg()
        .new_cl_draft(
            path,
            &session_id,
            "Pending upload",
            &from_hash,
            &user.username,
        )
        .await
        .map_err(ApiError::internal)?;

    // Create session record
    state
        .buck_stg()
        .create_session(&session_id, &user.username, path, &from_hash, expires_at)
        .await
        .map_err(ApiError::internal)?;

    // Return response
    let max_file_size = buck_config
        .and_then(|b| b.get_max_file_size_bytes().ok())
        .unwrap_or(100 * 1024 * 1024); // 100MB default
    let response = SessionResponse {
        session_id,
        expires_at: expires_at.to_rfc3339(),
        max_file_size,
        max_files: buck_config.map(|b| b.max_files).unwrap_or(1000),
        max_concurrent_uploads: buck_config.map(|b| b.max_concurrent_uploads).unwrap_or(5),
    };

    Ok(Json(CommonResult::success(Some(response))))
}

/// Upload file manifest
///
/// Submit file manifest and get list of files that need to be uploaded.
#[utoipa::path(
    post,
    params(("session_id", description = "Session ID")),
    path = "/session/{session_id}/manifest",
    request_body = ManifestPayload,
    responses(
        (status = 200, body = CommonResult<ManifestResponse>),
        (status = 400, description = "Invalid manifest format or empty"),
        (status = 404, description = "Session not found"),
        (status = 409, description = "Invalid session status"),
    ),
    tag = BUCK_TAG
)]
async fn upload_manifest(
    user: LoginUser,
    Path(session_id): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ManifestPayload>,
) -> Result<Json<CommonResult<ManifestResponse>>, ApiError> {
    // Validate session
    let session = validate_session(
        &state,
        &session_id,
        &user.username,
        &[session_status::CREATED],
    )
    .await?;

    // Check file count limit
    let config = state.storage.config();
    let max_files = config.buck.as_ref().map(|b| b.max_files).unwrap_or(1000);
    if payload.files.len() > max_files as usize {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "File count {} exceeds limit {}. Consider splitting into multiple uploads.",
            payload.files.len(),
            max_files
        )));
    }

    // Validate manifest
    if payload.files.is_empty() {
        return Err(ApiError::bad_request(anyhow::anyhow!("Empty file list")));
    }
    for file in &payload.files {
        validate_manifest_file(file)?;
    }

    // Get existing file hashes for manifest files only (avoid loading all files)
    let from_hash = session.from_hash.as_deref().unwrap_or("");
    let manifest_paths: Vec<PathBuf> = payload
        .files
        .iter()
        .map(|f| PathBuf::from(&f.path))
        .collect();
    let current_files = get_existing_file_hashes(&state, from_hash, &manifest_paths).await?;

    // Compare and determine changes
    let mut files_to_upload = Vec::new();
    let mut files_unchanged = 0u32;
    let mut upload_size = 0u64;
    let mut file_records = Vec::new();

    for file in &payload.files {
        let path = std::path::PathBuf::from(&file.path);
        let new_hash = parse_hash(&file.hash)?;

        let (upload_status, upload_reason, existing_blob_id) = match current_files.get(&path) {
            None => {
                // New file: needs upload
                files_to_upload.push(FileToUpload {
                    path: file.path.clone(),
                    reason: upload_reason::NEW.to_string(),
                });
                upload_size += file.size;
                (
                    upload_status::PENDING.to_string(),
                    Some(upload_reason::NEW.to_string()),
                    None,
                )
            }
            Some(old_hash) if *old_hash != new_hash => {
                // Modified file: needs upload
                files_to_upload.push(FileToUpload {
                    path: file.path.clone(),
                    reason: upload_reason::MODIFIED.to_string(),
                });
                upload_size += file.size;
                (
                    upload_status::PENDING.to_string(),
                    Some(upload_reason::MODIFIED.to_string()),
                    None,
                )
            }
            Some(old_hash) => {
                // Unchanged file: skip upload but save blob_id for tree building
                files_unchanged += 1;
                ("skipped".to_string(), None, Some(old_hash.to_string()))
            }
        };

        file_records.push(FileRecord {
            file_path: file.path.clone(),
            file_size: file.size as i64,
            file_hash: file.hash.clone(),
            file_mode: Some(file.mode.clone()), // mode is always present (defaults to "100644")
            upload_status,
            upload_reason,
            blob_id: existing_blob_id,
        });
    }

    // Batch insert file records
    const BATCH_SIZE: usize = 1000;
    let mut inserted_count = 0;
    for (chunk_idx, chunk) in file_records.chunks(BATCH_SIZE).enumerate() {
        match state
            .buck_stg()
            .batch_insert_files(&session_id, chunk.to_vec())
            .await
        {
            Ok(()) => {
                inserted_count += chunk.len();
                tracing::debug!(
                    "Buck upload batch insert: session={}, chunk={}, inserted={}, total_inserted={}",
                    session_id,
                    chunk_idx,
                    chunk.len(),
                    inserted_count
                );
            }
            Err(e) => {
                tracing::error!(
                    "Buck upload batch insert failed: session={}, chunk={}, inserted_so_far={}, total={}, error={}",
                    session_id,
                    chunk_idx,
                    inserted_count,
                    file_records.len(),
                    e
                );
                // Return error with context about partial success
                return Err(ApiError::internal(anyhow::anyhow!(
                    "Failed to insert file records (inserted {} of {} files)",
                    inserted_count,
                    file_records.len()
                )));
            }
        }
    }

    // Update session status
    state
        .buck_stg()
        .update_session_status(
            &session_id,
            session_status::MANIFEST_UPLOADED,
            payload.commit_message.as_deref(),
        )
        .await
        .map_err(ApiError::internal)?;

    // Return response
    let total_size: u64 = payload.files.iter().map(|f| f.size).sum();
    let response = ManifestResponse {
        total_files: payload.files.len() as u32,
        total_size,
        files_to_upload,
        files_unchanged,
        upload_size,
    };

    Ok(Json(CommonResult::success(Some(response))))
}

/// Validate session exists, belongs to user, not expired, and has correct status
async fn validate_session(
    state: &State<MonoApiServiceState>,
    session_id: &str,
    username: &str,
    allowed_statuses: &[&str],
) -> Result<callisto::buck_session::Model, ApiError> {
    let session = state
        .buck_stg()
        .get_session(session_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| {
            ApiError::with_status(StatusCode::NOT_FOUND, anyhow::anyhow!("Session not found"))
        })?;

    if session.user_id != username {
        return Err(ApiError::with_status(
            StatusCode::FORBIDDEN,
            anyhow::anyhow!("Session belongs to another user"),
        ));
    }

    if session.expires_at < chrono::Utc::now().naive_utc() {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            anyhow::anyhow!("Session expired"),
        ));
    }

    if !allowed_statuses.contains(&session.status.as_str()) {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            anyhow::anyhow!("Invalid session status: {}", session.status),
        ));
    }

    Ok(session)
}

/// Validate manifest file entry
fn validate_manifest_file(file: &ManifestFile) -> Result<(), ApiError> {
    // Check path format
    if file.path.starts_with('/') {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Path must not start with '/': {}",
            file.path
        )));
    }
    if file.path.contains('\\') {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Path must use '/' separator: {}",
            file.path
        )));
    }

    // Reject Windows absolute paths (e.g., "C:/Windows/..." or "C:\\Windows\\...")
    // This check works on all platforms, not just Windows
    if file.path.len() >= 2 {
        let first_two = &file.path[..2];
        if first_two
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false)
            && first_two.chars().nth(1) == Some(':')
        {
            return Err(ApiError::bad_request(anyhow::anyhow!(
                "Absolute path not allowed (Windows drive letter detected): {}",
                file.path
            )));
        }
    }

    // Security check: forbid .git directory (any level)
    // Matches: ".git/xxx" or "foo/.git/xxx" or "foo/bar/.git/xxx"
    if file.path.starts_with(".git/") || file.path.contains("/.git/") {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Forbidden path (.git directory not allowed): {}",
            file.path
        )));
    }

    // Security check: forbid path traversal
    if file.path.contains("..") {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Path traversal not allowed: {}",
            file.path
        )));
    }

    // Check hash format
    if !file.hash.starts_with("sha1:") {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Hash must start with 'sha1:': {}",
            file.hash
        )));
    }
    let hash_part = &file.hash[5..];
    // SHA1 hash must be 40 lowercase hex characters
    if hash_part.len() != 40
        || !hash_part
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
    {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Invalid hash format (must be 40 lowercase hex chars): {}",
            file.hash
        )));
    }

    // Check mode (mode is always present, defaults to "100644")
    if !["100644", "100755", "120000"].contains(&file.mode.as_str()) {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Invalid mode: {}",
            file.mode
        )));
    }

    // Check path length (prevent resource exhaustion)
    if file.path.len() > 4096 {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Path too long (max 4096 characters): {}",
            file.path
        )));
    }

    // Check nesting depth (prevent stack overflow)
    let path = std::path::PathBuf::from(&file.path);
    let depth = path.components().count();
    if depth > 100 {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Path nesting too deep (max 100 levels, got {}): {}",
            depth,
            file.path
        )));
    }

    // Normalize path and verify it matches original (prevents bypassing checks)
    // This catches paths like "a/./b", "a//b", etc.
    let normalized = path.components().collect::<std::path::PathBuf>();
    let normalized_str = normalized.to_string_lossy().replace('\\', "/");
    if normalized_str != file.path {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Path contains invalid components (normalized: {}): {}",
            normalized_str,
            file.path
        )));
    }

    // Check for empty segments or invalid components (e.g., "//", "/./")
    // This prevents paths like "a//b" or "a/./b" from bypassing validation
    if file.path.contains("//") || file.path.contains("/./") {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Path contains invalid segments: {}",
            file.path
        )));
    }

    Ok(())
}

/// Parse hash string (strip sha1: prefix)
fn parse_hash(hash: &str) -> Result<String, ApiError> {
    let hash_str = hash.strip_prefix("sha1:").unwrap_or(hash);
    Ok(hash_str.to_string())
}

/// Get existing file hashes for specific paths only (batch lookup)
///
/// This function queries only the files mentioned in the manifest, avoiding
/// the memory explosion issue of loading all files from a large repository.
///
/// # Arguments
/// * `state` - API service state
/// * `commit_hash` - Commit hash to query from
/// * `paths` - Only query these specific file paths
///
/// # Returns
/// HashMap mapping file paths to their blob hashes (as strings)
async fn get_existing_file_hashes(
    state: &State<MonoApiServiceState>,
    commit_hash: &str,
    paths: &[PathBuf],
) -> Result<HashMap<PathBuf, String>, ApiError> {
    if commit_hash.is_empty() || paths.is_empty() {
        return Ok(HashMap::new());
    }

    let mono_service = state.monorepo();

    // Use the batch query API for optimal performance
    let blob_ids = blob_ops::get_files_blob_ids(&mono_service, paths, Some(commit_hash))
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to batch query blob IDs: commit={}, paths_count={}, error={}",
                commit_hash,
                paths.len(),
                e
            );
            ApiError::internal(anyhow::anyhow!("Failed to query existing file hashes"))
        })?;

    // Convert SHA1 to String
    let result: HashMap<PathBuf, String> = blob_ids
        .into_iter()
        .map(|(path, sha1)| (path, sha1.to_string()))
        .collect();

    Ok(result)
}

/// Upload file
///
/// Upload a single file content. Can be called concurrently for different files.
#[utoipa::path(
    post,
    params(("session_id", description = "Session ID")),
    path = "/session/{session_id}/file",
    responses(
        (status = 200, body = CommonResult<FileUploadResponse>),
        (status = 400, description = "Invalid parameters or validation failed"),
        (status = 404, description = "Session not found or file not in manifest"),
        (status = 413, description = "File too large"),
        (status = 415, description = "Invalid Content-Type"),
    ),
    tag = BUCK_TAG
)]
async fn upload_file(
    user: LoginUser,
    Path(session_id): Path<String>,
    state: State<MonoApiServiceState>,
    headers: HeaderMap,
    req: Request<Body>,
) -> Result<Json<CommonResult<FileUploadResponse>>, ApiError> {
    use axum::body::to_bytes;
    use percent_encoding::percent_decode_str;

    // Parse file_size from header
    let file_size: u64 = headers
        .get("x-file-size")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| {
            ApiError::bad_request(anyhow::anyhow!("Missing or invalid X-File-Size header"))
        })?;

    // Validate file size limit BEFORE rate limiting decision
    let config = state.storage.config();
    let max_size = config
        .buck
        .as_ref()
        .and_then(|b| b.get_max_file_size_bytes().ok())
        .unwrap_or(100 * 1024 * 1024); // 100MB default
    if file_size > max_size {
        return Err(ApiError::with_status(
            StatusCode::PAYLOAD_TOO_LARGE,
            anyhow::anyhow!("File size {} exceeds limit {}", file_size, max_size),
        ));
    }

    // Rate limiting: acquire global upload semaphore (non-blocking)
    let _upload_permit = match state.buck_upload_semaphore.try_acquire() {
        Ok(permit) => permit,
        Err(TryAcquireError::NoPermits) => {
            tracing::warn!(
                "Buck upload rate limited (global): session={}, file_size={}, user={}",
                session_id,
                file_size,
                user.username
            );
            return Err(ApiError::with_status(
                StatusCode::TOO_MANY_REQUESTS,
                anyhow::anyhow!("Server is busy, please retry later"),
            ));
        }
        Err(TryAcquireError::Closed) => {
            tracing::error!("Buck upload semaphore closed unexpectedly");
            return Err(ApiError::internal(anyhow::anyhow!(
                "Upload semaphore closed"
            )));
        }
    };

    // Rate limiting: acquire large file semaphore if needed (non-blocking)
    let _large_file_permit = if file_size >= state.buck_large_file_threshold {
        match state.buck_large_file_semaphore.try_acquire() {
            Ok(permit) => Some(permit),
            Err(TryAcquireError::NoPermits) => {
                tracing::warn!(
                    "Buck upload rate limited (large file): session={}, file_size={}, threshold={}, user={}",
                    session_id,
                    file_size,
                    state.buck_large_file_threshold,
                    user.username
                );
                return Err(ApiError::with_status(
                    StatusCode::TOO_MANY_REQUESTS,
                    anyhow::anyhow!(
                        "Too many large file uploads in progress. File size: {} bytes, threshold: {} bytes",
                        file_size,
                        state.buck_large_file_threshold
                    ),
                ));
            }
            Err(TryAcquireError::Closed) => {
                tracing::error!("Buck upload large file semaphore closed unexpectedly");
                return Err(ApiError::internal(anyhow::anyhow!(
                    "Large file semaphore closed"
                )));
            }
        }
    } else {
        None
    };

    tracing::debug!(
        "Buck upload started: session={}, file_size={}, is_large_file={}, user={}",
        session_id,
        file_size,
        _large_file_permit.is_some(),
        user.username
    );

    // Validate Content-Type
    if let Some(ct) = headers.get("content-type")
        && ct != "application/octet-stream"
    {
        return Err(ApiError::with_status(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            anyhow::anyhow!("Content-Type must be application/octet-stream"),
        ));
    }

    // Parse remaining headers
    let file_path = headers
        .get("x-file-path")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::bad_request(anyhow::anyhow!("Missing X-File-Path header")))?;

    let file_path = percent_decode_str(file_path)
        .decode_utf8()
        .map_err(|e| ApiError::bad_request(anyhow::anyhow!("Invalid X-File-Path encoding: {}", e)))?
        .to_string();

    let file_hash = headers
        .get("x-file-hash")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Validate session
    let _session = validate_session(
        &state,
        &session_id,
        &user.username,
        &[session_status::MANIFEST_UPLOADED, session_status::UPLOADING],
    )
    .await?;

    // Verify file is in pending list
    let _file_record = state
        .buck_stg()
        .get_pending_file(&session_id, &file_path)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| {
            ApiError::with_status(
                StatusCode::NOT_FOUND,
                anyhow::anyhow!("File not in manifest or already uploaded: {}", file_path),
            )
        })?;

    // Read body (file size already validated before rate limiting)
    let body_bytes = to_bytes(req.into_body(), max_size as usize)
        .await
        .map_err(|e| ApiError::bad_request(anyhow::anyhow!("Failed to read body: {}", e)))?;

    // Validate size matches
    if body_bytes.len() as u64 != file_size {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Size mismatch: header says {}, got {}",
            file_size,
            body_bytes.len()
        )));
    }

    // Save blob to database
    let blob_hash = state
        .storage
        .raw_db_storage()
        .save_raw_blob_from_content(body_bytes.to_vec())
        .await
        .map_err(ApiError::internal)?;

    // Verify hash if provided
    let verified = if let Some(expected) = &file_hash {
        let expected = expected.strip_prefix("sha1:").unwrap_or(expected);
        if blob_hash != expected {
            return Err(ApiError::bad_request(anyhow::anyhow!(
                "Hash mismatch: expected {}, got {}",
                expected,
                blob_hash
            )));
        }
        Some(true)
    } else {
        None
    };

    // Mark file as uploaded
    let rows_affected = state
        .buck_stg()
        .mark_file_uploaded(&session_id, &file_path, &blob_hash)
        .await
        .map_err(ApiError::internal)?;

    if rows_affected == 0 {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            anyhow::anyhow!("File already uploaded or status changed"),
        ));
    }

    // Update session status to "uploading" if needed
    // Only update if current status is MANIFEST_UPLOADED to avoid race conditions
    // when multiple files are uploaded concurrently
    let current_status = _session.status.clone();
    if current_status == session_status::MANIFEST_UPLOADED {
        state
            .buck_stg()
            .update_session_status(&session_id, session_status::UPLOADING, None)
            .await
            .map_err(ApiError::internal)?;
    }

    // Return response
    Ok(Json(CommonResult::success(Some(FileUploadResponse {
        file_id: file_path,
        uploaded_size: file_size,
        verified,
    }))))
}

/// Complete upload
///
/// Complete the upload session, create Git commit, and activate CL.
/// Returns immediately - CI build is triggered asynchronously.
#[utoipa::path(
    post,
    params(("session_id", description = "Session ID")),
    path = "/session/{session_id}/complete",
    request_body = CompletePayload,
    responses(
        (status = 200, body = CommonResult<CompleteResponse>),
        (status = 400, description = "Files not fully uploaded"),
        (status = 404, description = "Session not found"),
        (status = 409, description = "Invalid session status"),
    ),
    tag = BUCK_TAG
)]
async fn complete_upload(
    user: LoginUser,
    Path(session_id): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<CompletePayload>,
) -> Result<Json<CommonResult<CompleteResponse>>, ApiError> {
    use callisto::sea_orm_active_enums::MergeStatusEnum;
    use ceres::api_service::buck_tree_builder::BuckCommitBuilder;
    use ceres::model::buck::FileChange;

    // Validate session
    let session = validate_session(
        &state,
        &session_id,
        &user.username,
        &[session_status::MANIFEST_UPLOADED, session_status::UPLOADING],
    )
    .await?;

    // Check if all files are uploaded
    let pending_count = state
        .buck_stg()
        .count_pending_files(&session_id)
        .await
        .map_err(ApiError::internal)?;

    if pending_count > 0 && !payload.skip_checks {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "{} file(s) not uploaded yet",
            pending_count
        )));
    }

    // Get all files (uploaded + skipped)
    let all_files = state
        .buck_stg()
        .get_all_files(&session_id)
        .await
        .map_err(ApiError::internal)?;

    // Verify all files have blob_id
    for file in &all_files {
        if file.blob_id.is_none() {
            return Err(ApiError::internal(anyhow::anyhow!(
                "Missing blob_id for file: {} (status: {})",
                file.file_path,
                file.upload_status
            )));
        }
    }

    // Get commit message
    let commit_message = payload
        .commit_message
        .or(session.commit_message.clone())
        .unwrap_or_else(|| "Upload via buck push".to_string());

    // Build commit BEFORE starting transaction
    let base_commit = session.from_hash.clone().unwrap_or_default();

    // Convert uploaded files to FileChange
    // Only files with upload_status::UPLOADED are needed to build the changes,
    // since skipped files already exist in the base commit tree and do not need to be updated.
    let file_changes: Vec<FileChange> = all_files
        .iter()
        .filter(|f| f.upload_status == upload_status::UPLOADED)
        .map(|f| {
            // Normalize blob_id
            let blob_id = f.blob_id.as_ref().unwrap();
            let normalized_blob_id = if blob_id.starts_with("sha1:") {
                blob_id.clone()
            } else {
                format!("sha1:{}", blob_id)
            };

            FileChange::new(
                f.file_path.clone(),
                normalized_blob_id,
                f.file_mode.clone().unwrap_or_else(|| "100644".to_string()),
            )
        })
        .collect();

    // Build commit using BuckCommitBuilder
    let commit_result = if file_changes.is_empty() {
        // No changes, use base commit
        None
    } else {
        let builder = BuckCommitBuilder::new(state.storage.mono_storage());
        let result = builder
            .build_commit(&base_commit, &file_changes, &commit_message)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Buck upload complete_upload build_commit failed: session={}, error={}",
                    session_id,
                    e
                );
                ApiError::internal(anyhow::anyhow!("Failed to build commit"))
            })?;
        Some(result)
    };

    // Begin database transaction for atomic commit
    let mono_storage = state.storage.mono_storage();
    let db = mono_storage.get_connection();
    let txn = db.begin().await.map_err(|e| {
        tracing::error!(
            "Buck upload complete_upload transaction begin failed: session={}, error={}",
            session_id,
            e
        );
        ApiError::internal(anyhow::anyhow!("Failed to begin transaction"))
    })?;

    // Get CL within transaction
    let cl = callisto::mega_cl::Entity::find()
        .filter(callisto::mega_cl::Column::Link.eq(&session_id))
        .one(&txn)
        .await
        .map_err(|e| {
            tracing::error!(
                "Buck upload complete_upload get CL in transaction failed: session={}, error={}",
                session_id,
                e
            );
            ApiError::internal(anyhow::anyhow!("Failed to get CL in transaction"))
        })?
        .ok_or_else(|| ApiError::not_found(anyhow::anyhow!("CL not found")))?;

    let commit_id = if let Some(result) = commit_result {
        // Save new trees
        // Use ON CONFLICT DO NOTHING
        if !result.new_tree_models.is_empty() {
            let tree_models: Vec<callisto::mega_tree::ActiveModel> = result
                .new_tree_models
                .into_iter()
                .map(|m| m.into_active_model())
                .collect();
            callisto::mega_tree::Entity::insert_many(tree_models)
                .on_conflict(
                    OnConflict::column(callisto::mega_tree::Column::TreeId)
                        .do_nothing()
                        .to_owned(),
                )
                .do_nothing()
                .exec(&txn)
                .await
                .map_err(|e| {
                    tracing::error!(
                        "Buck upload complete_upload save trees failed: session={}, error={}",
                        session_id,
                        e
                    );
                    ApiError::internal(anyhow::anyhow!("Failed to save trees"))
                })?;

            // Fault injection point: Trees saved, Commit not saved
            inject_fail!(
                injection::FAIL_TREE_SAVE,
                "TEST: Injecting Tree Save Failure (after save)",
                "Injected: Tree Save"
            );
        }

        // Save commit
        // Use ON CONFLICT DO NOTHING for idempotency
        let commit_model = result
            .commit
            .clone()
            .into_mega_model(git_internal::internal::metadata::EntryMeta::default())
            .into_active_model();
        callisto::mega_commit::Entity::insert(commit_model)
            .on_conflict(
                OnConflict::column(callisto::mega_commit::Column::CommitId)
                    .do_nothing()
                    .to_owned(),
            )
            .do_nothing()
            .exec(&txn)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Buck upload complete_upload save commit failed: session={}, error={}",
                    session_id,
                    e
                );
                ApiError::internal(anyhow::anyhow!("Failed to save commit"))
            })?;

        // Fault injection point: Commit saved, Ref not updated
        inject_fail!(
            injection::FAIL_COMMIT_SAVE,
            "TEST: Injecting Commit Save Failure (after save)",
            "Injected: Commit Save"
        );

        // Create or update CL ref
        let cl_ref_name = format!("refs/cl/{}", session_id);
        state
            .storage
            .mono_storage()
            .save_or_update_cl_ref_in_txn(
                &txn,
                &session.repo_path,
                &cl_ref_name,
                &result.commit_id,
                &result.tree_hash,
            )
            .await
            .map_err(|e| {
                tracing::error!(
                    "Buck upload complete_upload create CL ref failed: session={}, error={}",
                    session_id,
                    e
                );
                ApiError::internal(anyhow::anyhow!("Failed to create CL ref"))
            })?;

        // Fault injection point: Ref updated, CL not updated
        inject_fail!(
            injection::FAIL_REF_UPDATE,
            "TEST: Injecting Ref Update Failure (after save)",
            "Injected: Ref Update"
        );

        result.commit_id
    } else {
        // No changes, use base commit
        base_commit.clone()
    };

    // Update CL
    let mut cl_active = cl.clone().into_active_model();
    cl_active.from_hash = Set(base_commit.to_owned());
    cl_active.to_hash = Set(commit_id.to_owned());
    cl_active.status = Set(MergeStatusEnum::Open);
    cl_active.title = Set(commit_message.to_owned());
    cl_active.updated_at = Set(chrono::Utc::now().naive_utc());
    cl_active.update(&txn).await.map_err(|e| {
        tracing::error!(
            "Buck upload complete_upload update CL failed: session={}, cl_id={}, error={}",
            session_id,
            cl.id,
            e
        );
        ApiError::internal(anyhow::anyhow!("Failed to update CL"))
    })?;

    // Fault injection point: CL updated, Session not updated
    inject_fail!(
        injection::FAIL_CL_UPDATE,
        "TEST: Injecting CL Update Failure (after save)",
        "Injected: CL Update"
    );

    // Update session status to completed
    callisto::buck_session::Entity::update_many()
        .col_expr(
            callisto::buck_session::Column::Status,
            Expr::value(session_status::COMPLETED),
        )
        .col_expr(
            callisto::buck_session::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(callisto::buck_session::Column::SessionId.eq(&session_id))
        .exec(&txn)
        .await
        .map_err(|e| {
            tracing::error!(
                "Buck upload complete_upload update session failed: session={}, error={}",
                session_id,
                e
            );
            ApiError::internal(anyhow::anyhow!("Failed to update session"))
        })?;

    // Commit transaction
    txn.commit().await.map_err(|e| {
        tracing::error!(
            "Buck upload complete_upload transaction commit failed: session={}, commit_id={}, error={}",
            session_id,
            commit_id,
            e
        );
        ApiError::internal(anyhow::anyhow!("Failed to commit transaction"))
    })?;

    tracing::info!(
        "Buck upload complete_upload transaction committed: session={}, commit_id={}",
        session_id,
        commit_id
    );

    // Cleanup session files asynchronously (fire and forget)
    // File records are no longer needed after successful commit
    // This is a one-time cleanup task with retry mechanism for robustness
    let cleanup_storage = state.storage.buck_storage();
    let cleanup_session_id = session_id.clone();
    tokio::spawn(async move {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY_SECS: u64 = 5;

        for attempt in 1..=MAX_RETRIES {
            match cleanup_storage
                .delete_session_files(&cleanup_session_id)
                .await
            {
                Ok(count) => {
                    if count > 0 {
                        tracing::debug!(
                            "Buck upload cleanup: deleted {} file records for session {}",
                            count,
                            cleanup_session_id
                        );
                    }
                    return; // Success, exit retry loop
                }
                Err(e) => {
                    if attempt < MAX_RETRIES {
                        tracing::warn!(
                            "Buck upload cleanup failed for session {} (attempt {}/{}): {}. Retrying in {}s",
                            cleanup_session_id,
                            attempt,
                            MAX_RETRIES,
                            e,
                            RETRY_DELAY_SECS
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                    } else {
                        // Final attempt failed - log error but don't panic
                        // Background cleanup task will handle it later
                        tracing::error!(
                            "Buck upload cleanup failed for session {} after {} attempts: {}",
                            cleanup_session_id,
                            MAX_RETRIES,
                            e
                        );
                    }
                }
            }
        }
    });

    // Publish event asynchronously (non-blocking)
    let cl_link = session_id.clone();
    let commit_id_clone = commit_id.clone();
    let repo_path = session.repo_path.clone();
    let username = user.username.clone();
    tokio::spawn(async move {
        tracing::info!(
            "CL created event: link={}, commit={}, path={}, user={}",
            cl_link,
            commit_id_clone,
            repo_path,
            username
        );
        // TODO: Trigger CI build via Orion if configured
    });

    // Return response immediately (don't wait for CI)
    // Note: files_count only includes actually uploaded files, not skipped/unchanged files
    let uploaded_count = all_files
        .iter()
        .filter(|f| f.upload_status == upload_status::UPLOADED)
        .count();

    Ok(Json(CommonResult::success(Some(CompleteResponse {
        cl_id: cl.id,
        cl_link: session_id,
        commit_id,
        files_count: uploaded_count as u32,
        created_at: cl.created_at.and_utc().to_rfc3339(),
    }))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ceres::model::buck::ManifestFile;

    #[test]
    fn test_validate_rejects_absolute_path() {
        let file = ManifestFile {
            path: "/absolute/path.txt".to_string(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_err(), "Absolute path should be rejected");
    }

    #[test]
    fn test_validate_rejects_backslash() {
        let file = ManifestFile {
            path: "path\\to\\file.txt".to_string(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_err(), "Backslash separator should be rejected");
    }

    #[test]
    fn test_validate_rejects_windows_absolute_path() {
        let windows_paths = vec![
            "C:/Windows/System32/config/sam",
            "C:\\Windows\\System32\\config\\sam",
            "D:/Users/test.txt",
            "Z:/path/to/file",
            "A:/root",
        ];

        for path in windows_paths {
            let file = ManifestFile {
                path: path.to_string(),
                size: 100,
                hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
                mode: "100644".to_string(),
            };
            let result = validate_manifest_file(&file);
            assert!(
                result.is_err(),
                "Windows absolute path should be rejected: {}",
                path
            );
        }
    }

    #[test]
    fn test_validate_accepts_relative_path() {
        let file = ManifestFile {
            path: "relative/path.txt".to_string(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_ok(), "Relative path should be valid");
    }

    #[test]
    fn test_validate_rejects_git_directory_at_root() {
        let file = ManifestFile {
            path: ".git/config".to_string(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_err(), ".git directory at root should be rejected");
    }

    #[test]
    fn test_validate_rejects_git_directory_nested() {
        let file = ManifestFile {
            path: "submodule/.git/config".to_string(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_err(), "Nested .git directory should be rejected");
    }

    #[test]
    fn test_validate_rejects_git_directory_deeply_nested() {
        let file = ManifestFile {
            path: "a/b/c/.git/objects/pack".to_string(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_err(), "Deeply nested .git should be rejected");
    }

    #[test]
    fn test_validate_allows_gitignore_file() {
        // ".gitignore" is NOT ".git/" - should be allowed
        let file = ManifestFile {
            path: ".gitignore".to_string(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_ok(), ".gitignore file should be allowed");
    }

    #[test]
    fn test_validate_allows_gitkeep_file() {
        let file = ManifestFile {
            path: "empty_dir/.gitkeep".to_string(),
            size: 0,
            hash: "sha1:e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_ok(), ".gitkeep file should be allowed");
    }

    #[test]
    fn test_validate_rejects_path_traversal_simple() {
        let file = ManifestFile {
            path: "../etc/passwd".to_string(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_err(), "Path traversal should be rejected");
    }

    #[test]
    fn test_validate_rejects_path_traversal_in_middle() {
        let file = ManifestFile {
            path: "a/b/../../../etc/passwd".to_string(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(
            result.is_err(),
            "Path traversal in middle should be rejected"
        );
    }

    #[test]
    fn test_validate_rejects_hash_without_prefix() {
        let file = ManifestFile {
            path: "file.txt".to_string(),
            size: 100,
            hash: "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(), // Missing sha1:
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(
            result.is_err(),
            "Hash without sha1: prefix should be rejected"
        );
    }

    #[test]
    fn test_validate_rejects_hash_wrong_length() {
        let file = ManifestFile {
            path: "file.txt".to_string(),
            size: 100,
            hash: "sha1:abc123".to_string(), // Too short
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_err(), "Hash with wrong length should be rejected");
    }

    #[test]
    fn test_validate_rejects_hash_uppercase() {
        let file = ManifestFile {
            path: "file.txt".to_string(),
            size: 100,
            hash: "sha1:A94A8FE5CCB19BA61C4C0873D391E987982FBBD3".to_string(), // Uppercase
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_err(), "Uppercase hash should be rejected");
    }

    #[test]
    fn test_validate_rejects_hash_non_hex() {
        let file = ManifestFile {
            path: "file.txt".to_string(),
            size: 100,
            hash: "sha1:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_string(), // Invalid chars
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_err(), "Non-hex characters should be rejected");
    }

    #[test]
    fn test_validate_accepts_valid_hash() {
        let file = ManifestFile {
            path: "file.txt".to_string(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_ok(), "Valid hash should be accepted");
    }

    #[test]
    fn test_validate_accepts_valid_modes() {
        for mode in &["100644", "100755", "120000"] {
            let file = ManifestFile {
                path: "file.txt".to_string(),
                size: 100,
                hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
                mode: mode.to_string(),
            };
            let result = validate_manifest_file(&file);
            assert!(result.is_ok(), "Mode {} should be valid", mode);
        }
    }

    #[test]
    fn test_validate_rejects_invalid_mode() {
        let file = ManifestFile {
            path: "file.txt".to_string(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "777".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_err(), "Invalid mode should be rejected");
    }

    #[test]
    fn test_validate_accepts_default_mode() {
        let file = ManifestFile {
            path: "file.txt".to_string(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(result.is_ok(), "Default mode (100644) should be accepted");
    }

    #[test]
    fn test_validate_rejects_path_too_long() {
        // Create a path longer than 4096 characters
        let long_path = "a/".repeat(2500) + "file.txt";
        assert!(long_path.len() > 4096, "Test path should exceed 4096 chars");

        let file = ManifestFile {
            path: long_path,
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
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

        let file = ManifestFile {
            path: deep_path.clone(),
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
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

        let file = ManifestFile {
            path: limit_path,
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
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

        let file = ManifestFile {
            path: limit_path,
            size: 100,
            hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
            mode: "100644".to_string(),
        };
        let result = validate_manifest_file(&file);
        assert!(
            result.is_ok(),
            "Path with exactly 100 levels should be accepted"
        );
    }
}
