//! Buck Service Layer Tests
//!
//! # Current Status
//!
//! **All tests in this file are currently ignored (`#[ignore]`) due to database migration issues.**
//! The tests require database migrations that are not yet finalized.
//!
//! This file contains tests for BuckService

use bytes::Bytes;
use chrono::{Duration, Utc};
use common::config::BuckConfig;
use git_internal::internal::object::blob::Blob;
use jupiter::service::buck_service::{BuckService, ManifestPayload};
use jupiter::service::cl_service::CLService;
use jupiter::storage::buck_storage::{session_status, upload_reason, upload_status};
use jupiter::tests::test_storage;
use serial_test::serial;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::Semaphore;

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a BuckService with real database connection for testing.
///
/// This function is currently unused because all tests are ignored.
/// It will be used again once database migration issues are resolved.
#[allow(dead_code)]
async fn create_test_buck_service_with_db()
-> (BuckService, jupiter::storage::Storage, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;

    // Get BaseStorage from app_service (through any storage that has base)
    let base = storage.buck_storage().base.clone();

    let upload_semaphore = Arc::new(Semaphore::new(10));
    let large_file_semaphore = Arc::new(Semaphore::new(5));
    let buck_config = BuckConfig::default();
    // Use real CLService backed by the same BaseStorage to avoid disconnected DB in tests
    let cl_service = CLService::new(base.clone());

    let buck_service = BuckService::new(
        base,
        cl_service,
        upload_semaphore,
        large_file_semaphore,
        buck_config,
    )
    .expect("Failed to create BuckService");

    (buck_service, storage, temp_dir)
}

// ============================================================================
// Section 1: BuckService::validate_session Tests
// ============================================================================

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_validate_session_not_found() {
    let (service, _storage, _temp_dir) = create_test_buck_service_with_db().await;

    let result = service
        .validate_session("NONEXISTENT", "test_user", &[session_status::CREATED])
        .await;

    assert!(result.is_err(), "Session not found should return error");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Session not found"),
        "Error should indicate session not found"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_validate_session_wrong_user() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Create session for user1
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "user1", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Try to validate as user2
    let result = service
        .validate_session(session_id, "user2", &[session_status::CREATED])
        .await;

    assert!(result.is_err(), "Wrong user should return error");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("belongs to another user"),
        "Error should indicate wrong user"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_validate_session_expired() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Create expired session
    let session_id = "TEST1234";
    let expired_at = Utc::now() - Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expired_at)
        .await
        .unwrap();

    let result = service
        .validate_session(session_id, "test_user", &[session_status::CREATED])
        .await;

    assert!(result.is_err(), "Expired session should return error");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("expired"),
        "Error should indicate session expired"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_validate_session_invalid_status() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Create session with CREATED status
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Try to validate with wrong status
    let result = service
        .validate_session(
            session_id,
            "test_user",
            &[session_status::MANIFEST_UPLOADED],
        )
        .await;

    assert!(result.is_err(), "Invalid status should return error");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Invalid session status"),
        "Error should indicate invalid status"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_validate_session_success() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Create valid session
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    let result = service
        .validate_session(session_id, "test_user", &[session_status::CREATED])
        .await;

    assert!(result.is_ok(), "Valid session should succeed");
    let session = result.unwrap();
    assert_eq!(session.session_id, session_id);
    assert_eq!(session.user_id, "test_user");
}

// ============================================================================
// Section 2: BuckService::create_session Tests
// ============================================================================

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_create_session_success() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Setup: Create a repo with a main ref
    let repo_path = "/test/repo";
    let from_hash = "abc123def456";

    // Create main ref (simulating existing repo)
    // Note: get_main_ref looks for MEGA_BRANCH_NAME, which is typically "refs/heads/mega"
    use common::utils::MEGA_BRANCH_NAME;
    storage
        .mono_storage()
        .save_or_update_cl_ref(repo_path, MEGA_BRANCH_NAME, from_hash, from_hash)
        .await
        .unwrap();

    // Create session
    let result = service
        .create_session("test_user", repo_path, from_hash.to_string())
        .await;

    if let Err(e) = &result {
        panic!("Session creation failed: {}", e);
    }
    let response = result.unwrap();

    // Verify session was created
    let session = storage
        .buck_storage()
        .get_session(&response.cl_link)
        .await
        .unwrap()
        .expect("Session should exist");

    assert_eq!(session.user_id, "test_user");
    assert_eq!(session.repo_path, repo_path);
    assert_eq!(session.from_hash, Some(from_hash.to_string()));
    assert_eq!(session.status, session_status::CREATED);

    // Verify CL was created
    let cl = storage
        .cl_storage()
        .get_cl(&response.cl_link)
        .await
        .unwrap();
    assert!(cl.is_some(), "Draft CL should be created");
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_create_session_expires_at_calculation() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Setup: Create a repo with a main ref
    let repo_path = "/test/repo";
    let from_hash = "abc123def456";

    use common::utils::MEGA_BRANCH_NAME;
    storage
        .mono_storage()
        .save_or_update_cl_ref(repo_path, MEGA_BRANCH_NAME, from_hash, from_hash)
        .await
        .unwrap();

    let before = Utc::now();
    let result = service
        .create_session("test_user", repo_path, from_hash.to_string())
        .await;
    let after = Utc::now();

    if let Err(e) = &result {
        panic!("Session creation failed: {}", e);
    }
    let response = result.unwrap();

    // Parse expires_at from response
    let expires_at = chrono::DateTime::parse_from_rfc3339(&response.expires_at)
        .unwrap()
        .with_timezone(&Utc);

    // Verify expires_at is in the future
    assert!(expires_at > before, "Expires at should be in the future");
    assert!(
        expires_at < after + Duration::hours(2),
        "Expires at should be reasonable"
    );

    // Verify it matches session's expires_at
    let session = storage
        .buck_storage()
        .get_session(&response.cl_link)
        .await
        .unwrap()
        .unwrap();

    let session_expires_at = chrono::DateTime::from_naive_utc_and_offset(session.expires_at, Utc);

    // Allow small time difference (within 1 second)
    let diff = (expires_at - session_expires_at).num_seconds().abs();
    assert!(
        diff < 2,
        "Response expires_at should match session expires_at"
    );
}

// ============================================================================
// Section 3: BuckService::process_manifest Tests
// ============================================================================
// Note: These tests require Git operations to get existing file hashes.
// For now, we'll test with empty existing_file_hashes (all files are new).

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_process_manifest_empty_list() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Create session
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Try to process empty manifest
    let payload = ManifestPayload {
        files: vec![],
        commit_message: None,
    };
    let existing_file_hashes = HashMap::new();

    let result = service
        .process_manifest("test_user", session_id, payload, existing_file_hashes)
        .await;

    assert!(result.is_err(), "Empty manifest should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Empty file list"),
        "Error should indicate empty list"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_process_manifest_idempotency() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Create session
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Create manifest
    let payload = ManifestPayload {
        files: vec![jupiter::service::buck_service::ManifestFile {
            path: "file1.txt".to_string(),
            size: 100,
            hash: "sha1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        }],
        commit_message: None,
    };
    let existing_file_hashes = HashMap::new();

    // Process manifest first time
    let result1 = service
        .process_manifest(
            "test_user",
            session_id,
            payload.clone(),
            existing_file_hashes.clone(),
        )
        .await;
    assert!(result1.is_ok(), "First manifest should succeed");

    // Reset session status to CREATED to allow reprocessing
    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::CREATED, None)
        .await
        .unwrap();

    // Process same manifest again (idempotency)
    // Note: This may fail with "None of the records are inserted" if all files already exist
    // This is expected behavior for idempotency - the operation is effectively a no-op
    let result2 = service
        .process_manifest("test_user", session_id, payload, existing_file_hashes)
        .await;

    // Idempotency: Either succeeds (if some files are new) or fails gracefully (if all files exist)
    // Both cases are acceptable for idempotent operations
    if result2.is_err() {
        let err_msg = result2.unwrap_err().to_string();
        // Accept "None of the records are inserted" as valid idempotent behavior
        assert!(
            err_msg.contains("None of the records are inserted")
                || err_msg.contains("not inserted"),
            "Idempotent operation should either succeed or fail gracefully, but got: {}",
            err_msg
        );
    }

    // Verify files were not duplicated (should still be 1 file)
    let files = storage
        .buck_storage()
        .get_all_files(session_id)
        .await
        .unwrap();
    assert_eq!(files.len(), 1, "Files should not be duplicated");
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_process_manifest_duplicate_paths() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Create session
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Create manifest with duplicate paths
    let payload = ManifestPayload {
        files: vec![
            jupiter::service::buck_service::ManifestFile {
                path: "file1.txt".to_string(),
                size: 100,
                hash: "sha1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            },
            jupiter::service::buck_service::ManifestFile {
                path: "file1.txt".to_string(), // Duplicate
                size: 200,
                hash: "sha1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            },
        ],
        commit_message: None,
    };
    let existing_file_hashes = HashMap::new();

    let result = service
        .process_manifest("test_user", session_id, payload, existing_file_hashes)
        .await;

    assert!(result.is_err(), "Duplicate paths should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Duplicate file path"),
        "Error should indicate duplicate path"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_process_manifest_success() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Create session
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Create manifest with new files
    let payload = ManifestPayload {
        files: vec![
            jupiter::service::buck_service::ManifestFile {
                path: "file1.txt".to_string(),
                size: 100,
                hash: "sha1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            },
            jupiter::service::buck_service::ManifestFile {
                path: "file2.txt".to_string(),
                size: 200,
                hash: "sha1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            },
        ],
        commit_message: Some("Test commit".to_string()),
    };
    let existing_file_hashes = HashMap::new(); // All files are new

    let result = service
        .process_manifest("test_user", session_id, payload, existing_file_hashes)
        .await;

    assert!(result.is_ok(), "Valid manifest should succeed");
    let response = result.unwrap();

    assert_eq!(response.total_files, 2);
    assert_eq!(response.files_to_upload.len(), 2);
    assert_eq!(response.files_unchanged, 0);
    assert_eq!(response.upload_size, 300);

    // Verify files were inserted
    let files = storage
        .buck_storage()
        .get_all_files(session_id)
        .await
        .unwrap();
    assert_eq!(files.len(), 2);

    // Verify session status updated
    let session = storage
        .buck_storage()
        .get_session(session_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(session.status, session_status::MANIFEST_UPLOADED);
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_process_manifest_with_existing_files() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Create session
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Create manifest with mixed files
    let payload = ManifestPayload {
        files: vec![
            jupiter::service::buck_service::ManifestFile {
                path: "file1.txt".to_string(),
                size: 100,
                hash: "sha1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            },
            jupiter::service::buck_service::ManifestFile {
                path: "file2.txt".to_string(),
                size: 200,
                hash: "sha1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            },
            jupiter::service::buck_service::ManifestFile {
                path: "file3.txt".to_string(),
                size: 300,
                hash: "sha1:cccccccccccccccccccccccccccccccccccccccc".to_string(),
            },
        ],
        commit_message: None,
    };

    // file2.txt exists with same hash (unchanged)
    // file3.txt exists with different hash (modified)
    let mut existing_file_hashes = HashMap::new();
    existing_file_hashes.insert(
        PathBuf::from("file2.txt"),
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
    );
    existing_file_hashes.insert(
        PathBuf::from("file3.txt"),
        "old_hash_old_hash_old_hash_old_hash_old_hash".to_string(),
    );

    let result = service
        .process_manifest("test_user", session_id, payload, existing_file_hashes)
        .await;

    assert!(result.is_ok());
    let response = result.unwrap();

    assert_eq!(response.total_files, 3);
    assert_eq!(response.files_to_upload.len(), 2); // file1 (new) + file3 (modified)
    assert_eq!(response.files_unchanged, 1); // file2 (unchanged)
    assert_eq!(response.upload_size, 400); // file1 (100) + file3 (300)
}

// ============================================================================
// Section 4: BuckService::upload_file Tests
// ============================================================================

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_upload_file_success() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Setup: Create session and add file to manifest
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Prepare file content and calculate hash
    let file_content_bytes = b"Hello, World!".repeat(8); // 104 bytes
    let blob = Blob::from_content_bytes(file_content_bytes.clone());
    let actual_hash = blob.id.to_string();

    // Add file to manifest with correct hash
    storage
        .buck_storage()
        .batch_insert_files(
            session_id,
            vec![jupiter::storage::buck_storage::FileRecord {
                file_path: "file1.txt".to_string(),
                file_size: 104,
                file_hash: format!("sha1:{}", actual_hash),
                file_mode: Some("100644".to_string()),
                upload_status: upload_status::PENDING.to_string(),
                upload_reason: Some(upload_reason::NEW.to_string()),
                blob_id: None,
            }],
        )
        .await
        .unwrap();

    // Update session status to MANIFEST_UPLOADED
    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::MANIFEST_UPLOADED, None)
        .await
        .unwrap();

    // Upload file
    let file_content = Bytes::from(file_content_bytes);
    let result = service
        .upload_file(
            "test_user",
            session_id,
            "file1.txt",
            104,
            None, // Let service verify against manifest hash
            file_content,
        )
        .await;

    assert!(result.is_ok(), "File upload should succeed");
    let response = result.unwrap();

    assert_eq!(response.file_path, "file1.txt");
    assert_eq!(response.uploaded_size, 104);
    assert_eq!(response.verified, Some(true));

    // Verify file was marked as uploaded
    let files = storage
        .buck_storage()
        .get_all_files(session_id)
        .await
        .unwrap();
    let file = files.iter().find(|f| f.file_path == "file1.txt").unwrap();
    assert_eq!(file.upload_status, upload_status::UPLOADED);
    assert!(file.blob_id.is_some());
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_upload_file_invalid_session_status() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Create session with CREATED status (not MANIFEST_UPLOADED)
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Try to upload file
    let file_content = Bytes::from("test content");
    let result = service
        .upload_file("test_user", session_id, "file1.txt", 12, None, file_content)
        .await;

    assert!(result.is_err(), "Invalid session status should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Invalid session status"),
        "Error should indicate invalid status"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_upload_file_not_in_manifest() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Create session and set status
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::MANIFEST_UPLOADED, None)
        .await
        .unwrap();

    // Try to upload file not in manifest
    let file_content = Bytes::from("test content");
    let result = service
        .upload_file(
            "test_user",
            session_id,
            "file_not_in_manifest.txt",
            12,
            None,
            file_content,
        )
        .await;

    assert!(result.is_err(), "File not in manifest should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not in manifest"),
        "Error should indicate file not in manifest"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_upload_file_size_exceeds_limit() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Setup: Create session and add file to manifest
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Get max_file_size from service
    let max_file_size = service.max_file_size();
    let oversized = max_file_size + 1;

    // Add file to manifest
    let file_content_bytes = vec![0u8; oversized as usize];
    let blob = Blob::from_content_bytes(file_content_bytes.clone());
    let actual_hash = blob.id.to_string();

    storage
        .buck_storage()
        .batch_insert_files(
            session_id,
            vec![jupiter::storage::buck_storage::FileRecord {
                file_path: "file1.txt".to_string(),
                file_size: oversized as i64,
                file_hash: format!("sha1:{}", actual_hash),
                file_mode: Some("100644".to_string()),
                upload_status: upload_status::PENDING.to_string(),
                upload_reason: Some(upload_reason::NEW.to_string()),
                blob_id: None,
            }],
        )
        .await
        .unwrap();

    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::MANIFEST_UPLOADED, None)
        .await
        .unwrap();

    // Try to upload oversized file
    let file_content = Bytes::from(file_content_bytes);
    let result = service
        .upload_file(
            "test_user",
            session_id,
            "file1.txt",
            oversized,
            None,
            file_content,
        )
        .await;

    assert!(result.is_err(), "Oversized file should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("exceeds limit"),
        "Error should indicate size limit exceeded"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_upload_file_size_mismatch() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Setup: Create session and add file to manifest
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    let file_content_bytes = b"test content".to_vec();
    let blob = Blob::from_content_bytes(file_content_bytes.clone());
    let actual_hash = blob.id.to_string();

    // Add file to manifest with size 100, but actual content is 12 bytes
    storage
        .buck_storage()
        .batch_insert_files(
            session_id,
            vec![jupiter::storage::buck_storage::FileRecord {
                file_path: "file1.txt".to_string(),
                file_size: 100, // Wrong size
                file_hash: format!("sha1:{}", actual_hash),
                file_mode: Some("100644".to_string()),
                upload_status: upload_status::PENDING.to_string(),
                upload_reason: Some(upload_reason::NEW.to_string()),
                blob_id: None,
            }],
        )
        .await
        .unwrap();

    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::MANIFEST_UPLOADED, None)
        .await
        .unwrap();

    // Try to upload with size mismatch
    let file_content = Bytes::from(file_content_bytes);
    let result = service
        .upload_file(
            "test_user",
            session_id,
            "file1.txt",
            100, // Header says 100
            None,
            file_content, // But content is 12 bytes
        )
        .await;

    assert!(result.is_err(), "Size mismatch should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Size mismatch"),
        "Error should indicate size mismatch"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_upload_file_hash_mismatch() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Setup: Create session and add file to manifest
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Add file to manifest with hash A
    storage
        .buck_storage()
        .batch_insert_files(
            session_id,
            vec![jupiter::storage::buck_storage::FileRecord {
                file_path: "file1.txt".to_string(),
                file_size: 12,
                file_hash: "sha1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(), // Wrong hash
                file_mode: Some("100644".to_string()),
                upload_status: upload_status::PENDING.to_string(),
                upload_reason: Some(upload_reason::NEW.to_string()),
                blob_id: None,
            }],
        )
        .await
        .unwrap();

    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::MANIFEST_UPLOADED, None)
        .await
        .unwrap();

    // Upload file with different content (different hash)
    let file_content = Bytes::from(b"test content".to_vec());
    let result = service
        .upload_file("test_user", session_id, "file1.txt", 12, None, file_content)
        .await;

    assert!(result.is_err(), "Hash mismatch should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Hash mismatch"),
        "Error should indicate hash mismatch"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_upload_file_already_uploaded() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Setup: Create session and add file to manifest
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    let file_content_bytes = b"test content".to_vec();
    let blob = Blob::from_content_bytes(file_content_bytes.clone());
    let actual_hash = blob.id.to_string();

    // Add file to manifest and mark as uploaded
    storage
        .buck_storage()
        .batch_insert_files(
            session_id,
            vec![jupiter::storage::buck_storage::FileRecord {
                file_path: "file1.txt".to_string(),
                file_size: 12,
                file_hash: format!("sha1:{}", actual_hash),
                file_mode: Some("100644".to_string()),
                upload_status: upload_status::UPLOADED.to_string(), // Already uploaded
                upload_reason: Some(upload_reason::NEW.to_string()),
                blob_id: Some(actual_hash.clone()),
            }],
        )
        .await
        .unwrap();

    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::MANIFEST_UPLOADED, None)
        .await
        .unwrap();

    // Try to upload already uploaded file
    let file_content = Bytes::from(file_content_bytes.clone());
    let result = service
        .upload_file("test_user", session_id, "file1.txt", 12, None, file_content)
        .await;

    assert!(result.is_err(), "Already uploaded file should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not in manifest") || err.to_string().contains("not pending"),
        "Error should indicate file not pending"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_upload_file_hash_verification_success() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Setup: Create session and add file to manifest
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    let file_content_bytes = b"test content".to_vec();
    let blob = Blob::from_content_bytes(file_content_bytes.clone());
    let actual_hash = blob.id.to_string();

    // Add file to manifest
    storage
        .buck_storage()
        .batch_insert_files(
            session_id,
            vec![jupiter::storage::buck_storage::FileRecord {
                file_path: "file1.txt".to_string(),
                file_size: 12,
                file_hash: format!("sha1:{}", actual_hash),
                file_mode: Some("100644".to_string()),
                upload_status: upload_status::PENDING.to_string(),
                upload_reason: Some(upload_reason::NEW.to_string()),
                blob_id: None,
            }],
        )
        .await
        .unwrap();

    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::MANIFEST_UPLOADED, None)
        .await
        .unwrap();

    // Upload file with explicit hash verification
    let file_content = Bytes::from(file_content_bytes);
    let result = service
        .upload_file(
            "test_user",
            session_id,
            "file1.txt",
            12,
            Some(&format!("sha1:{}", actual_hash)), // Explicit hash
            file_content,
        )
        .await;

    assert!(
        result.is_ok(),
        "File upload with correct hash should succeed"
    );
    let response = result.unwrap();
    assert_eq!(
        response.verified,
        Some(true),
        "Hash verification should pass"
    );
}

// ============================================================================
// Section 5: BuckService::complete_upload Tests
// ============================================================================

use git_internal::hash::SHA1;
use git_internal::internal::metadata::EntryMeta;
use git_internal::internal::object::commit::Commit;
use git_internal::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use jupiter::service::buck_service::{CommitArtifacts, CompletePayload};
use jupiter::utils::converter::IntoMegaModel;
use sea_orm::IntoActiveModel;
use std::str::FromStr;

/// Helper: Create a simple commit artifact for testing
fn create_test_commit_artifacts(
    commit_id: &str,
    tree_hash: &str,
    _repo_path: &str,
) -> CommitArtifacts {
    // Create a simple tree with one blob
    let tree = Tree::from_tree_items(vec![TreeItem {
        mode: TreeItemMode::Blob,
        id: SHA1::from_str(tree_hash).unwrap(),
        name: "test.txt".to_string(),
    }])
    .unwrap();

    let tree_model = tree.into_mega_model(EntryMeta::default());

    // Create a commit
    let commit = Commit::from_tree_id(SHA1::from_str(tree_hash).unwrap(), vec![], "Test commit");

    let commit_model = commit.into_mega_model(EntryMeta::default());

    CommitArtifacts {
        commit_id: commit_id.to_string(),
        tree_hash: tree_hash.to_string(),
        new_tree_models: vec![tree_model.into_active_model()],
        commit_model: commit_model.into_active_model(),
    }
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_complete_upload_success() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Setup: Create session, manifest, and upload files
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Create draft CL (required by complete_upload)
    storage
        .cl_storage()
        .new_cl_draft("/test/repo", session_id, "Test CL", "hash123", "test_user")
        .await
        .unwrap();

    // Add files and mark as uploaded
    let blob_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    storage
        .buck_storage()
        .batch_insert_files(
            session_id,
            vec![jupiter::storage::buck_storage::FileRecord {
                file_path: "test.txt".to_string(),
                file_size: 100,
                file_hash: format!("sha1:{}", blob_hash),
                file_mode: Some("100644".to_string()),
                upload_status: upload_status::UPLOADED.to_string(),
                upload_reason: Some(upload_reason::NEW.to_string()),
                blob_id: Some(blob_hash.to_string()),
            }],
        )
        .await
        .unwrap();

    // Save blob to database
    storage
        .raw_db_storage()
        .save_raw_blob_from_content(vec![0u8; 100])
        .await
        .unwrap();

    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::UPLOADING, None)
        .await
        .unwrap();

    // Create commit artifacts
    let commit_id = "cccccccccccccccccccccccccccccccccccccccc";
    let tree_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let artifacts = create_test_commit_artifacts(commit_id, tree_hash, "/test/repo");

    // Complete upload
    let payload = CompletePayload {
        commit_message: Some("Test commit".to_string()),
    };

    let result = service
        .complete_upload("test_user", session_id, payload, Some(artifacts))
        .await;

    if let Err(e) = &result {
        panic!("Complete upload failed: {}", e);
    }
    let response = result.unwrap();
    assert_eq!(response.commit_id, commit_id);

    // Verify session status updated
    let session = storage
        .buck_storage()
        .get_session(session_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(session.status, session_status::COMPLETED);
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_complete_upload_invalid_session_status() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Create session with CREATED status (not MANIFEST_UPLOADED or UPLOADING)
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Create draft CL (required by complete_upload)
    storage
        .cl_storage()
        .new_cl_draft("/test/repo", session_id, "Test CL", "hash123", "test_user")
        .await
        .unwrap();

    let payload = CompletePayload {
        commit_message: None,
    };

    let result = service
        .complete_upload("test_user", session_id, payload, None)
        .await;

    assert!(result.is_err(), "Invalid session status should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Invalid session status"),
        "Error should indicate invalid status"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_complete_upload_pending_files() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Setup: Create session with pending files
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Create draft CL (required by complete_upload)
    storage
        .cl_storage()
        .new_cl_draft("/test/repo", session_id, "Test CL", "hash123", "test_user")
        .await
        .unwrap();

    // Add file with PENDING status
    storage
        .buck_storage()
        .batch_insert_files(
            session_id,
            vec![jupiter::storage::buck_storage::FileRecord {
                file_path: "test.txt".to_string(),
                file_size: 100,
                file_hash: "sha1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                file_mode: Some("100644".to_string()),
                upload_status: upload_status::PENDING.to_string(),
                upload_reason: Some(upload_reason::NEW.to_string()),
                blob_id: None,
            }],
        )
        .await
        .unwrap();

    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::MANIFEST_UPLOADED, None)
        .await
        .unwrap();

    let payload = CompletePayload {
        commit_message: None,
    };

    let result = service
        .complete_upload("test_user", session_id, payload, None)
        .await;

    assert!(result.is_err(), "Pending files should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not fully uploaded"),
        "Error should indicate pending files"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_complete_upload_missing_blob_id() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Setup: Create session with file missing blob_id
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Create draft CL (required by complete_upload)
    storage
        .cl_storage()
        .new_cl_draft("/test/repo", session_id, "Test CL", "hash123", "test_user")
        .await
        .unwrap();

    // Add file without blob_id
    storage
        .buck_storage()
        .batch_insert_files(
            session_id,
            vec![jupiter::storage::buck_storage::FileRecord {
                file_path: "test.txt".to_string(),
                file_size: 100,
                file_hash: "sha1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                file_mode: Some("100644".to_string()),
                upload_status: upload_status::UPLOADED.to_string(),
                upload_reason: Some(upload_reason::NEW.to_string()),
                blob_id: None, // Missing blob_id
            }],
        )
        .await
        .unwrap();

    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::UPLOADING, None)
        .await
        .unwrap();

    let payload = CompletePayload {
        commit_message: None,
    };

    let result = service
        .complete_upload("test_user", session_id, payload, None)
        .await;

    assert!(result.is_err(), "Missing blob_id should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not fully uploaded"),
        "Error should indicate missing blob_id"
    );
}

#[tokio::test]
#[ignore] // TODO: Re-enable after database migration issues are resolved
#[serial]
async fn test_complete_upload_idempotency() {
    let (service, storage, _temp_dir) = create_test_buck_service_with_db().await;

    // Setup: Create session and complete upload
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    storage
        .buck_storage()
        .create_session(session_id, "test_user", "/test/repo", "hash123", expires_at)
        .await
        .unwrap();

    // Create draft CL (required by complete_upload)
    storage
        .cl_storage()
        .new_cl_draft("/test/repo", session_id, "Test CL", "hash123", "test_user")
        .await
        .unwrap();

    let blob_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    storage
        .buck_storage()
        .batch_insert_files(
            session_id,
            vec![jupiter::storage::buck_storage::FileRecord {
                file_path: "test.txt".to_string(),
                file_size: 100,
                file_hash: format!("sha1:{}", blob_hash),
                file_mode: Some("100644".to_string()),
                upload_status: upload_status::UPLOADED.to_string(),
                upload_reason: Some(upload_reason::NEW.to_string()),
                blob_id: Some(blob_hash.to_string()),
            }],
        )
        .await
        .unwrap();

    storage
        .raw_db_storage()
        .save_raw_blob_from_content(vec![0u8; 100])
        .await
        .unwrap();

    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::UPLOADING, None)
        .await
        .unwrap();

    let commit_id = "cccccccccccccccccccccccccccccccccccccccc";
    let tree_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let artifacts = create_test_commit_artifacts(commit_id, tree_hash, "/test/repo");

    let payload = CompletePayload {
        commit_message: Some("Test commit".to_string()),
    };

    // First complete
    let result1 = service
        .complete_upload(
            "test_user",
            session_id,
            payload.clone(),
            Some(artifacts.clone()),
        )
        .await;
    assert!(result1.is_ok(), "First complete should succeed");

    // Reset session status to allow retry
    storage
        .buck_storage()
        .update_session_status_with_pool(session_id, session_status::UPLOADING, None)
        .await
        .unwrap();

    // Second complete (idempotency - should succeed due to ON CONFLICT DO NOTHING)
    let result2 = service
        .complete_upload("test_user", session_id, payload, Some(artifacts))
        .await;
    assert!(result2.is_ok(), "Idempotent complete should succeed");
}
