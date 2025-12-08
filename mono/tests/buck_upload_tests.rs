//! Comprehensive Buck Upload Tests
//!
//! This file contains all Buck upload related tests, organized into sections:
//! 1. Storage Layer Tests - BuckStorage and RawDbStorage operations
//! 2. UTF-8 Path Tests - Unicode path handling and validation
//! 3. Skipped Files Tests - Incremental upload behavior
//! 4. Tree Builder Tests - Directory grouping and tree construction
//! 5. Tree Builder Integration Tests - Complete tree building and commit creation flow
//!
//! Note: Tests use #[serial] to ensure database isolation.

use chrono::{Duration, Utc};
use jupiter::storage::buck_storage::{FileRecord, session_status, upload_reason, upload_status};
use jupiter::tests::test_storage;
use serial_test::serial;
use tempfile::tempdir;

use ceres::api_service::buck_tree_builder::BuckCommitBuilder;
use ceres::model::buck::{FileChange, ManifestFile};
use git_internal::hash::SHA1;
use git_internal::internal::metadata::EntryMeta;
use git_internal::internal::object::commit::Commit;
use git_internal::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use jupiter::storage::base_storage::StorageConnector;
use jupiter::utils::converter::IntoMegaModel;
use sea_orm::IntoActiveModel;
use std::path::PathBuf;
use std::str::FromStr;

// ============================================================================
// Section 1: Storage Layer Tests
// ============================================================================
// These tests verify BuckStorage and RawDbStorage operations work correctly.

#[tokio::test]
#[serial]
async fn test_create_and_get_session() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;
    let buck_storage = storage.buck_storage();

    let session_id = "TEST1234";
    let user_id = "test_user";
    let repo_path = "/test/repo";
    let from_hash = "abc123";
    let expires_at = Utc::now() + Duration::hours(1);

    // Create
    let result = buck_storage
        .create_session(session_id, user_id, repo_path, from_hash, expires_at)
        .await;
    assert!(result.is_ok(), "Session creation should succeed");

    // Get
    let session = buck_storage.get_session(session_id).await.unwrap();
    assert!(session.is_some(), "Session should exist");
    let session = session.unwrap();
    assert_eq!(session.session_id, session_id);
    assert_eq!(session.user_id, user_id);
    assert_eq!(session.repo_path, repo_path);
    assert_eq!(session.status, session_status::CREATED);
    assert_eq!(session.from_hash, Some(from_hash.to_string()));
}

#[tokio::test]
#[serial]
async fn test_batch_insert_and_get_files() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;
    let buck_storage = storage.buck_storage();

    // Setup session
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    buck_storage
        .create_session(session_id, "user", "/repo", "hash", expires_at)
        .await
        .unwrap();

    // Insert files
    let records = vec![
        FileRecord {
            file_path: "file1.txt".to_string(),
            file_size: 100,
            file_hash: "sha1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            file_mode: Some("100644".to_string()),
            upload_status: upload_status::PENDING.to_string(),
            upload_reason: Some(upload_reason::NEW.to_string()),
            blob_id: None,
        },
        FileRecord {
            file_path: "file2.txt".to_string(),
            file_size: 200,
            file_hash: "sha1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            file_mode: Some("100644".to_string()),
            upload_status: "skipped".to_string(),
            upload_reason: None,
            blob_id: Some("existing_blob".to_string()),
        },
    ];

    buck_storage
        .batch_insert_files(session_id, records)
        .await
        .unwrap();

    let files = buck_storage.get_all_files(session_id).await.unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].file_path, "file1.txt");
    assert_eq!(files[1].file_path, "file2.txt");
}

#[tokio::test]
#[serial]
async fn test_mark_file_uploaded() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;
    let buck_storage = storage.buck_storage();

    // Setup
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    buck_storage
        .create_session(session_id, "user", "/repo", "hash", expires_at)
        .await
        .unwrap();

    buck_storage
        .batch_insert_files(
            session_id,
            vec![FileRecord {
                file_path: "file1.txt".to_string(),
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

    // Mark as uploaded
    let affected = buck_storage
        .mark_file_uploaded(session_id, "file1.txt", "new_blob_id")
        .await
        .unwrap();
    assert_eq!(affected, 1, "One file should be updated");

    // Verify status changed
    let files = buck_storage.get_uploaded_files(session_id).await.unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].blob_id, Some("new_blob_id".to_string()));
    assert_eq!(files[0].upload_status, upload_status::UPLOADED);
}

#[tokio::test]
#[serial]
async fn test_count_pending_files() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;
    let buck_storage = storage.buck_storage();

    // Setup
    let session_id = "TEST1234";
    let expires_at = Utc::now() + Duration::hours(1);
    buck_storage
        .create_session(session_id, "user", "/repo", "hash", expires_at)
        .await
        .unwrap();

    buck_storage
        .batch_insert_files(
            session_id,
            vec![
                FileRecord {
                    file_path: "file1.txt".to_string(),
                    file_size: 100,
                    file_hash: "sha1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                    file_mode: Some("100644".to_string()),
                    upload_status: upload_status::PENDING.to_string(),
                    upload_reason: Some(upload_reason::NEW.to_string()),
                    blob_id: None,
                },
                FileRecord {
                    file_path: "file2.txt".to_string(),
                    file_size: 200,
                    file_hash: "sha1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                    file_mode: Some("100644".to_string()),
                    upload_status: upload_status::PENDING.to_string(),
                    upload_reason: Some(upload_reason::MODIFIED.to_string()),
                    blob_id: None,
                },
            ],
        )
        .await
        .unwrap();

    let count = buck_storage.count_pending_files(session_id).await.unwrap();
    assert_eq!(count, 2, "Should have 2 pending files");

    // Mark one as uploaded
    buck_storage
        .mark_file_uploaded(session_id, "file1.txt", "blob_id")
        .await
        .unwrap();

    let count = buck_storage.count_pending_files(session_id).await.unwrap();
    assert_eq!(count, 1, "Should have 1 pending file after upload");
}

#[tokio::test]
#[serial]
async fn test_save_raw_blob_from_content() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;
    let raw_storage = storage.raw_db_storage();

    let content = b"Hello, World!".to_vec();
    let hash = raw_storage
        .save_raw_blob_from_content(content.clone())
        .await
        .unwrap();

    // Verify hash format (40 lowercase hex)
    assert_eq!(hash.len(), 40);
    assert!(
        hash.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
    );

    // Verify can retrieve
    let blob = raw_storage.get_raw_blob_by_hash(&hash).await.unwrap();
    assert!(blob.is_some());
    assert_eq!(blob.unwrap().data, Some(content));
}

#[tokio::test]
#[serial]
async fn test_delete_expired_sessions() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;
    let buck_storage = storage.buck_storage();

    // Create expired session (not completed)
    let expired_at = Utc::now() - Duration::hours(1);
    buck_storage
        .create_session("EXPIRED1", "user", "/repo", "hash", expired_at)
        .await
        .unwrap();

    // Create valid session
    let valid_at = Utc::now() + Duration::hours(1);
    buck_storage
        .create_session("VALID001", "user", "/repo", "hash", valid_at)
        .await
        .unwrap();

    // Create completed session (should not be deleted within retention period)
    let completed_at = Utc::now() - Duration::hours(1);
    buck_storage
        .create_session("COMPLETED", "user", "/repo", "hash", completed_at)
        .await
        .unwrap();
    buck_storage
        .update_session_status("COMPLETED", session_status::COMPLETED, None)
        .await
        .unwrap();

    // Delete expired with 7 days retention
    let deleted = buck_storage.delete_expired_sessions(7).await.unwrap();
    assert_eq!(deleted, 1, "Should delete 1 expired session");

    // Verify
    assert!(
        buck_storage
            .get_session("EXPIRED1")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        buck_storage
            .get_session("VALID001")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        buck_storage
            .get_session("COMPLETED")
            .await
            .unwrap()
            .is_some()
    );
}

// ============================================================================
// Section 2: UTF-8 Path Tests
// ============================================================================
// These tests verify Unicode path handling and validation.

// Helper: Path validation function
fn validate_path(path: &str) -> Result<(), String> {
    if path.starts_with('/') {
        return Err(format!("Path must not start with '/': {}", path));
    }
    if path.contains('\\') {
        return Err(format!("Path must use '/' separator: {}", path));
    }
    if path.starts_with(".git/") || path.contains("/.git/") {
        return Err(format!(
            "Forbidden path (.git directory not allowed): {}",
            path
        ));
    }
    if path.contains("..") {
        return Err(format!("Path traversal not allowed: {}", path));
    }
    Ok(())
}

// Helper: Hash validation function
fn validate_hash(hash: &str) -> Result<(), String> {
    if !hash.starts_with("sha1:") {
        return Err(format!("Hash must start with 'sha1:': {}", hash));
    }
    let hash_part = &hash[5..];
    if hash_part.len() != 40
        || !hash_part
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
    {
        return Err(format!("Invalid hash format: {}", hash));
    }
    Ok(())
}

#[test]
fn test_unicode_paths_validation() {
    // Chinese characters
    assert!(validate_path("src/测试目录/文件.txt").is_ok());

    // Emoji
    assert!(validate_path("docs/🚀rocket/README.md").is_ok());

    // Mixed Unicode
    assert!(validate_path("测试/🎉/日本語/ファイル.txt").is_ok());

    // Spaces
    assert!(validate_path("my folder/sub folder/file name.txt").is_ok());
}

#[test]
fn test_special_chars_in_path() {
    // Special characters (but not dangerous ones)
    let path = "folder-name_v1.0/file@2023!.txt";
    assert!(validate_path(path).is_ok());
}

#[test]
fn test_hidden_files_allowed() {
    let paths = [
        ".gitignore",
        ".env",
        ".config/settings.json",
        "src/.hidden_file",
    ];
    for path in paths {
        assert!(
            validate_path(path).is_ok(),
            "Hidden file {} should be valid",
            path
        );
    }
}

#[test]
fn test_git_directory_forbidden() {
    let invalid_paths = [".git/config", "src/.git/HEAD", ".git/objects/ab/cd"];
    for path in invalid_paths {
        assert!(
            validate_path(path).is_err(),
            "Path {} should be forbidden",
            path
        );
    }
}

#[test]
fn test_path_traversal_forbidden() {
    let invalid_paths = ["../etc/passwd", "src/../../config", "a/../b"];
    for path in invalid_paths {
        assert!(
            validate_path(path).is_err(),
            "Path {} should be forbidden",
            path
        );
    }
}

#[test]
fn test_valid_hashes() {
    let valid_hashes = [
        "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3",
        "sha1:0000000000000000000000000000000000000000",
        "sha1:ffffffffffffffffffffffffffffffffffffffff",
        "sha1:e69de29bb2d1d6434b8b29ae775ad8c2e48c5391", // Empty blob hash
    ];
    for hash in valid_hashes {
        assert!(validate_hash(hash).is_ok(), "Hash {} should be valid", hash);
    }
}

#[test]
fn test_invalid_hashes() {
    let invalid_hashes = [
        "sha1:ABC",                                                                // Too short
        "sha1:A94A8FE5CCB19BA61C4C0873D391E987982FBBD3",                           // Uppercase
        "md5:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3",                            // Wrong prefix
        "sha256:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3a94a8fe5ccb19ba61c4c0873", // Wrong length
    ];
    for hash in invalid_hashes {
        assert!(
            validate_hash(hash).is_err(),
            "Hash {} should be invalid",
            hash
        );
    }
}

#[test]
fn test_manifest_file_with_unicode() {
    // Chinese path
    let file1 = ManifestFile {
        path: "中文目录/子目录/文件.txt".to_string(),
        size: 100,
        hash: "sha1:a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
        mode: "100644".to_string(),
    };
    assert!(validate_path(&file1.path).is_ok());
    assert!(validate_hash(&file1.hash).is_ok());

    // Emoji path
    let file2 = ManifestFile {
        path: "🎮games/🕹️/save.dat".to_string(),
        size: 1024,
        hash: "sha1:b94a8fe5ccb19ba61c4c0873d391e987982fbbd4".to_string(),
        mode: "100644".to_string(),
    };
    assert!(validate_path(&file2.path).is_ok());
    assert!(validate_hash(&file2.hash).is_ok());
}

// ============================================================================
// Section 3: Skipped Files Tests
// ============================================================================
// These tests verify that files marked as "skipped" (unchanged) are correctly
// retained in the new commit when completing an upload.

/// Helper: Create a complete upload scenario with files
async fn setup_upload_scenario(
    storage: &jupiter::storage::Storage,
    repo_path: &str,
    user_id: &str,
    files: Vec<(&str, &str, &str)>, // (path, hash, status)
) -> String {
    use callisto::entity_ext::generate_link;

    let session_id = generate_link();
    let from_hash = "47cf563739054567a35782e1f84ab110bfa35134";
    let expires_at = Utc::now() + Duration::hours(1);

    // Create session
    storage
        .buck_storage()
        .create_session(&session_id, user_id, repo_path, from_hash, expires_at)
        .await
        .unwrap();

    // Insert file records
    let records: Vec<_> = files
        .into_iter()
        .map(|(path, hash, status)| FileRecord {
            file_path: path.to_string(),
            file_size: 100,
            file_hash: format!("sha1:{}", hash),
            file_mode: Some("100644".to_string()),
            upload_status: status.to_string(),
            upload_reason: if status == upload_status::PENDING {
                Some(upload_reason::NEW.to_string())
            } else {
                None
            },
            blob_id: if status == "skipped" {
                Some(hash.to_string())
            } else {
                None
            },
        })
        .collect();

    storage
        .buck_storage()
        .batch_insert_files(&session_id, records)
        .await
        .unwrap();

    // Mark pending files as uploaded
    for file in storage
        .buck_storage()
        .get_all_files(&session_id)
        .await
        .unwrap()
    {
        if file.upload_status == upload_status::PENDING {
            storage
                .buck_storage()
                .mark_file_uploaded(&session_id, &file.file_path, &file.file_hash[5..])
                .await
                .unwrap();
        }
    }

    session_id
}

#[tokio::test]
#[serial]
async fn test_skipped_files_retained_in_new_commit() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;

    let repo_path = "/projects/test";
    let user_id = "test_user";

    // Push 1: Upload file_a.txt
    let session1 = setup_upload_scenario(
        &storage,
        repo_path,
        user_id,
        vec![(
            "file_a.txt",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            upload_status::PENDING,
        )],
    )
    .await;

    let files1 = storage
        .buck_storage()
        .get_all_files(&session1)
        .await
        .unwrap();
    assert_eq!(files1.len(), 1);
    assert_eq!(files1[0].file_path, "file_a.txt");
    assert_eq!(files1[0].upload_status, upload_status::UPLOADED);

    // Push 2: Upload file_b.txt, file_a.txt unchanged (skipped)
    let session2 = setup_upload_scenario(
        &storage,
        repo_path,
        user_id,
        vec![
            (
                "file_a.txt",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "skipped",
            ),
            (
                "file_b.txt",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                upload_status::PENDING,
            ),
        ],
    )
    .await;

    // Verify manifest shows file_a as skipped
    let files2 = storage
        .buck_storage()
        .get_all_files(&session2)
        .await
        .unwrap();
    assert_eq!(files2.len(), 2);

    let file_a = files2.iter().find(|f| f.file_path == "file_a.txt").unwrap();
    assert_eq!(file_a.upload_status, "skipped");
    assert_eq!(
        file_a.blob_id,
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
    );

    let file_b = files2.iter().find(|f| f.file_path == "file_b.txt").unwrap();
    assert_eq!(file_b.upload_status, upload_status::UPLOADED);
}

#[tokio::test]
#[serial]
async fn test_mixed_skipped_and_uploaded_in_same_directory() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;

    let repo_path = "/projects/test";
    let user_id = "test_user";

    // Setup: Mixed scenario in src/ directory
    let session = setup_upload_scenario(
        &storage,
        repo_path,
        user_id,
        vec![
            (
                "src/old.txt",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "skipped",
            ),
            (
                "src/new.txt",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                upload_status::PENDING,
            ),
            (
                "src/another_old.txt",
                "cccccccccccccccccccccccccccccccccccccccc",
                "skipped",
            ),
        ],
    )
    .await;

    let files = storage
        .buck_storage()
        .get_all_files(&session)
        .await
        .unwrap();

    assert_eq!(files.len(), 3);

    // Verify skipped files
    let skipped: Vec<_> = files
        .iter()
        .filter(|f| f.upload_status == "skipped")
        .collect();
    assert_eq!(skipped.len(), 2);

    // Verify uploaded file
    let uploaded: Vec<_> = files
        .iter()
        .filter(|f| f.upload_status == upload_status::UPLOADED)
        .collect();
    assert_eq!(uploaded.len(), 1);
    assert_eq!(uploaded[0].file_path, "src/new.txt");
}

// ============================================================================
// Section 4: Tree Builder Tests
// ============================================================================
// These tests verify directory grouping and tree construction logic.

#[test]
fn test_file_change_construction() {
    let change = FileChange {
        path: "src/main.rs".to_string(),
        blob_id: "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".to_string(),
        mode: "100644".to_string(),
    };

    assert_eq!(change.path, "src/main.rs");
    assert_eq!(change.mode, "100644");
}

#[test]
fn test_file_change_modes() {
    // Regular file
    let regular = FileChange {
        path: "file.txt".to_string(),
        blob_id: "aaa".to_string(),
        mode: "100644".to_string(),
    };
    assert_eq!(regular.mode, "100644");

    // Executable
    let executable = FileChange {
        path: "script.sh".to_string(),
        blob_id: "bbb".to_string(),
        mode: "100755".to_string(),
    };
    assert_eq!(executable.mode, "100755");

    // Symlink
    let symlink = FileChange {
        path: "link".to_string(),
        blob_id: "ccc".to_string(),
        mode: "120000".to_string(),
    };
    assert_eq!(symlink.mode, "120000");
}

fn group_files_by_directory(
    files: &[FileChange],
) -> std::collections::HashMap<PathBuf, Vec<FileChange>> {
    use std::collections::HashMap;

    let mut groups: HashMap<PathBuf, Vec<FileChange>> = HashMap::new();

    for file in files {
        let path = std::path::Path::new(&file.path);
        let parent = path.parent().unwrap_or(std::path::Path::new(""));

        // Ensure all intermediate directories are included
        let mut current = PathBuf::new();
        for component in parent.components() {
            current.push(component);
            groups.entry(current.clone()).or_default();
        }

        groups
            .entry(parent.to_path_buf())
            .or_default()
            .push(file.clone());
    }

    groups
}

#[test]
fn test_group_files_nested_directories() {
    let files = vec![FileChange {
        path: "a/b/c/file.txt".to_string(),
        blob_id: "aaa".to_string(),
        mode: "100644".to_string(),
    }];

    let groups = group_files_by_directory(&files);

    // Should have all intermediate directories
    assert!(groups.contains_key(&PathBuf::from("a")));
    assert!(groups.contains_key(&PathBuf::from("a/b")));
    assert!(groups.contains_key(&PathBuf::from("a/b/c")));

    // File should be in a/b/c
    assert_eq!(groups.get(&PathBuf::from("a/b/c")).unwrap().len(), 1);
}

#[test]
fn test_group_files_mixed_depths() {
    let files = vec![
        FileChange {
            path: "root.txt".to_string(),
            blob_id: "aaa".to_string(),
            mode: "100644".to_string(),
        },
        FileChange {
            path: "a/nested.txt".to_string(),
            blob_id: "bbb".to_string(),
            mode: "100644".to_string(),
        },
        FileChange {
            path: "a/b/deep.txt".to_string(),
            blob_id: "ccc".to_string(),
            mode: "100644".to_string(),
        },
    ];

    let groups = group_files_by_directory(&files);

    // Root should have root.txt
    assert_eq!(groups.get(&PathBuf::from("")).unwrap().len(), 1);

    // a should have nested.txt
    assert_eq!(groups.get(&PathBuf::from("a")).unwrap().len(), 1);

    // a/b should have deep.txt
    assert_eq!(groups.get(&PathBuf::from("a/b")).unwrap().len(), 1);
}

#[test]
fn test_directory_sorting_for_tree_hash() {
    let mut dirs = [
        PathBuf::from("a/b/c"),
        PathBuf::from("a/b"),
        PathBuf::from("a"),
        PathBuf::from("z"),
        PathBuf::from(""),
    ];

    // Sort by depth (deepest first), then by name
    dirs.sort_by(|a, b| {
        let depth_a = a.components().count();
        let depth_b = b.components().count();
        depth_b.cmp(&depth_a).then_with(|| a.cmp(b))
    });

    // Deepest directories should be processed first
    assert_eq!(dirs[0], PathBuf::from("a/b/c"));
    assert_eq!(dirs[1], PathBuf::from("a/b"));
    assert_eq!(dirs[2], PathBuf::from("a"));
    assert_eq!(dirs[3], PathBuf::from("z"));
    assert_eq!(dirs[4], PathBuf::from("")); // Root is last
}

#[test]
fn test_sibling_directories() {
    let files = vec![
        FileChange {
            path: "dir1/file1.txt".to_string(),
            blob_id: "aaa".to_string(),
            mode: "100644".to_string(),
        },
        FileChange {
            path: "dir2/file2.txt".to_string(),
            blob_id: "bbb".to_string(),
            mode: "100644".to_string(),
        },
    ];

    let groups = group_files_by_directory(&files);

    assert!(groups.contains_key(&PathBuf::from("dir1")));
    assert!(groups.contains_key(&PathBuf::from("dir2")));
    assert_eq!(groups.get(&PathBuf::from("dir1")).unwrap().len(), 1);
    assert_eq!(groups.get(&PathBuf::from("dir2")).unwrap().len(), 1);
}

#[test]
fn test_unicode_paths_in_tree_groups() {
    let files = vec![
        FileChange {
            path: "中文/文件.txt".to_string(),
            blob_id: "aaa".to_string(),
            mode: "100644".to_string(),
        },
        FileChange {
            path: "🎮/🕹️/game.dat".to_string(),
            blob_id: "bbb".to_string(),
            mode: "100644".to_string(),
        },
    ];

    let groups = group_files_by_directory(&files);

    assert!(groups.contains_key(&PathBuf::from("中文")));
    assert!(groups.contains_key(&PathBuf::from("🎮")));
    assert!(groups.contains_key(&PathBuf::from("🎮/🕹️")));
}

// ============================================================================
// Section 5: Tree Builder Integration Tests
// ============================================================================
// These tests verify the complete tree building and commit creation flow
// using actual database operations.

/// Helper: Create a base commit and root tree in the database for testing
async fn create_base_commit_and_tree(storage: &jupiter::storage::Storage) -> (String, String) {
    let mono_storage = storage.mono_storage();

    // Save the .gitkeep blob to database first (Git doesn't allow empty trees)
    // This ensures the blob exists when the tree references it
    let empty_blob_hash = storage
        .raw_db_storage()
        .save_raw_blob_from_content(vec![]) // Empty content for .gitkeep
        .await
        .unwrap();

    // Create a root tree with a placeholder file
    let root_tree = Tree::from_tree_items(vec![TreeItem {
        mode: TreeItemMode::Blob,
        id: SHA1::from_str(&empty_blob_hash).unwrap(),
        name: ".gitkeep".to_string(),
    }])
    .unwrap();

    // Save the root tree to database
    let tree_model = root_tree.clone().into_mega_model(EntryMeta::default());
    mono_storage
        .batch_save_model(vec![tree_model.into_active_model()], None)
        .await
        .unwrap();

    // Create a base commit pointing to the root tree
    let base_commit = Commit::from_tree_id(
        root_tree.id,
        vec![], // No parent (initial commit)
        "Initial commit for testing",
    );

    // Save the base commit to database
    mono_storage
        .save_mega_commits(vec![base_commit.clone()], None)
        .await
        .unwrap();

    (base_commit.id.to_string(), root_tree.id.to_string())
}

/// Helper: Create FileChange with actual blob saved to database
async fn create_file_change_with_blob(
    storage: &jupiter::storage::Storage,
    path: &str,
    content: &str,
) -> FileChange {
    // Save blob to database
    let blob_hash = storage
        .raw_db_storage()
        .save_raw_blob_from_content(content.as_bytes().to_vec())
        .await
        .unwrap();

    FileChange::new(
        path.to_string(),
        format!("sha1:{}", blob_hash),
        "100644".to_string(),
    )
}

#[tokio::test]
#[serial]
async fn test_build_tree_with_changes_basic() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;
    let builder = BuckCommitBuilder::new(storage.mono_storage());

    // Create base commit and tree
    let (base_commit_hash, _base_tree_hash) = create_base_commit_and_tree(&storage).await;

    // Create file change
    let file_change = create_file_change_with_blob(&storage, "test.txt", "Hello, World!").await;

    // Build tree with changes
    let result = builder
        .build_tree_with_changes(&base_commit_hash, &[file_change])
        .await
        .unwrap();

    // Verify results
    assert_ne!(
        result.tree_hash.to_string(),
        base_commit_hash,
        "New tree should be different from base"
    );
    assert!(
        !result.new_trees.is_empty(),
        "Should create at least one new tree"
    );

    // Verify root tree contains the file
    let root_tree = &result.root_tree;
    let test_file = root_tree
        .tree_items
        .iter()
        .find(|item| item.name == "test.txt");
    assert!(test_file.is_some(), "Root tree should contain test.txt");
    assert_eq!(
        test_file.unwrap().mode,
        TreeItemMode::Blob,
        "test.txt should be a blob"
    );
}

#[tokio::test]
#[serial]
async fn test_build_tree_with_changes_nested_directories() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;
    let builder = BuckCommitBuilder::new(storage.mono_storage());

    // Create base commit
    let (base_commit_hash, _) = create_base_commit_and_tree(&storage).await;

    // Create nested file
    let file_change =
        create_file_change_with_blob(&storage, "a/b/c/file.txt", "Nested content").await;

    // Build tree
    let result = builder
        .build_tree_with_changes(&base_commit_hash, &[file_change])
        .await
        .unwrap();

    // Verify nested structure
    // Should have trees for: root, a, a/b, a/b/c
    assert!(
        result.new_trees.len() >= 3,
        "Should create trees for intermediate directories"
    );

    // Verify root tree contains 'a'
    let root_tree = &result.root_tree;
    let a_dir = root_tree
        .tree_items
        .iter()
        .find(|item| item.name == "a" && item.mode == TreeItemMode::Tree);
    assert!(a_dir.is_some(), "Root should contain 'a' directory");
}

#[tokio::test]
#[serial]
async fn test_build_tree_with_changes_multiple_files() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;
    let builder = BuckCommitBuilder::new(storage.mono_storage());

    // Create base commit
    let (base_commit_hash, _) = create_base_commit_and_tree(&storage).await;

    // Create multiple files
    let files = vec![
        create_file_change_with_blob(&storage, "root.txt", "Root file").await,
        create_file_change_with_blob(&storage, "src/main.rs", "fn main() {}").await,
        create_file_change_with_blob(&storage, "src/lib.rs", "pub mod lib;").await,
        create_file_change_with_blob(&storage, "docs/readme.md", "# Readme").await,
    ];

    // Build tree
    let result = builder
        .build_tree_with_changes(&base_commit_hash, &files)
        .await
        .unwrap();

    // Verify all files are in correct trees
    let root_tree = &result.root_tree;

    // Verify root.txt is in root tree
    let root_file = root_tree
        .tree_items
        .iter()
        .find(|item| item.name == "root.txt");
    assert!(root_file.is_some(), "Root tree should contain root.txt");

    // Verify src directory is in root tree
    let src_dir = root_tree
        .tree_items
        .iter()
        .find(|item| item.name == "src" && item.mode == TreeItemMode::Tree);
    assert!(src_dir.is_some(), "Root tree should contain src directory");

    // Verify docs directory is in root tree
    let docs_dir = root_tree
        .tree_items
        .iter()
        .find(|item| item.name == "docs" && item.mode == TreeItemMode::Tree);
    assert!(
        docs_dir.is_some(),
        "Root tree should contain docs directory"
    );

    // Verify we have trees for src and docs directories
    assert!(
        result.new_trees.len() >= 2,
        "Should create trees for src and docs directories"
    );
}

#[tokio::test]
#[serial]
async fn test_build_tree_with_changes_empty_files() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;
    let builder = BuckCommitBuilder::new(storage.mono_storage());

    // Create base commit
    let (base_commit_hash, _base_tree_hash) = create_base_commit_and_tree(&storage).await;

    // Build tree with empty files
    let result = builder
        .build_tree_with_changes(&base_commit_hash, &[])
        .await
        .unwrap();

    // Verify tree is unchanged
    // Note: The base tree has a .gitkeep file, so the tree structure should be the same
    // (with .gitkeep file)
    assert_eq!(
        result.root_tree.tree_items.len(),
        1,
        "Tree should still have .gitkeep file"
    );
    assert!(
        result.new_trees.is_empty(),
        "No new trees should be created for empty files"
    );
}

#[tokio::test]
#[serial]
async fn test_build_commit_integration() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;
    let builder = BuckCommitBuilder::new(storage.mono_storage());

    // Create base commit
    let (base_commit_hash, _) = create_base_commit_and_tree(&storage).await;

    // Create file change
    let file_change = create_file_change_with_blob(&storage, "new_file.txt", "New content").await;

    // Build commit
    let commit_message = "Test commit";
    let result = builder
        .build_commit(&base_commit_hash, &[file_change], commit_message)
        .await
        .unwrap();

    // Verify commit
    assert!(
        !result.commit_id.is_empty(),
        "Commit ID should not be empty"
    );
    assert_eq!(
        result.commit.message, commit_message,
        "Commit message should match"
    );
    assert_eq!(
        result.commit.parent_commit_ids.len(),
        1,
        "Should have one parent"
    );
    assert_eq!(
        result.commit.parent_commit_ids[0].to_string(),
        base_commit_hash,
        "Parent should be base commit"
    );

    // Verify tree models have commit_id set
    assert!(
        !result.new_tree_models.is_empty(),
        "Should have tree models"
    );
    for tree_model in &result.new_tree_models {
        assert_eq!(
            tree_model.commit_id, result.commit_id,
            "Tree model commit_id should match commit"
        );
    }

    // Verify commit tree_hash matches
    assert_eq!(
        result.tree_hash,
        result.commit.tree_id.to_string(),
        "Commit tree_id should match tree_hash"
    );
}

#[tokio::test]
#[serial]
async fn test_build_tree_with_changes_existing_directory() {
    let temp_dir = tempdir().unwrap();
    let storage = test_storage(temp_dir.path()).await;
    let builder = BuckCommitBuilder::new(storage.mono_storage());

    // Create base commit with existing structure: src/main.rs
    let (base_commit_hash, _base_tree_hash) = create_base_commit_and_tree(&storage).await;

    // First, create a tree with src/main.rs
    let main_rs_blob = create_file_change_with_blob(&storage, "src/main.rs", "fn main() {}").await;
    let first_result = builder
        .build_tree_with_changes(&base_commit_hash, &[main_rs_blob])
        .await
        .unwrap();

    // Save all new trees from first result to database (simulating a previous commit)
    let first_tree_models: Vec<_> = first_result
        .new_trees
        .iter()
        .map(|tree| {
            tree.clone()
                .into_mega_model(EntryMeta::default())
                .into_active_model()
        })
        .collect();
    storage
        .mono_storage()
        .batch_save_model(first_tree_models, None)
        .await
        .unwrap();

    // Create a commit pointing to the first tree
    let first_commit = Commit::from_tree_id(
        first_result.tree_hash,
        vec![SHA1::from_str(&base_commit_hash).unwrap()],
        "First commit with src/main.rs",
    );
    storage
        .mono_storage()
        .save_mega_commits(vec![first_commit.clone()], None)
        .await
        .unwrap();

    // Add src/lib.rs (new file in existing directory)
    let lib_rs_blob = create_file_change_with_blob(&storage, "src/lib.rs", "pub mod lib;").await;
    let second_result = builder
        .build_tree_with_changes(&first_commit.id.to_string(), &[lib_rs_blob])
        .await
        .unwrap();

    // Verify src tree contains both files
    // We need to find the src tree in the new trees
    let src_tree = second_result.new_trees.iter().find(|tree| {
        // Check if this tree contains lib.rs
        tree.tree_items
            .iter()
            .any(|item| item.name == "lib.rs" && item.mode == TreeItemMode::Blob)
    });

    assert!(src_tree.is_some(), "Should have a tree containing lib.rs");

    let src_tree = src_tree.unwrap();
    let main_rs_item = src_tree
        .tree_items
        .iter()
        .find(|item| item.name == "main.rs");
    let lib_rs_item = src_tree
        .tree_items
        .iter()
        .find(|item| item.name == "lib.rs");

    assert!(
        main_rs_item.is_some(),
        "src tree should contain main.rs from previous commit"
    );
    assert!(lib_rs_item.is_some(), "src tree should contain lib.rs");
}
