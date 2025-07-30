use libra::command::{add, commit, init, remove};
use libra::utils::test;
use serial_test::serial;
use tempfile::tempdir;
use std::fs;
use std::path::Path;

async fn test_remove_file() {
    println!("\n\x1b[1mTest remove file functionality.\x1b[0m");

    // Create a test file
    test::ensure_file("test_file.txt", Some("test content"));
    
    // Add file to index
    let add_args = add::AddArgs {
        all: false,
        update: false,
        verbose: false,
        pathspec: vec!["test_file.txt".to_string()],
        dry_run: false,
        refresh: false,
        ignore_errors: false,
    };
    add::execute(add_args).await;

    // Remove file from index and filesystem
    let remove_args = remove::RemoveArgs {
        pathspec: vec!["test_file.txt".to_string()],
        cached: false,
        recursive: false,
        force: false,
    };
    let _ = remove::execute(remove_args);

    // Verify file is removed from filesystem
    assert!(!Path::new("test_file.txt").exists());
}

async fn test_remove_cached_file() {
    println!("\n\x1b[1mTest remove cached file functionality.\x1b[0m");

    // Create a test file
    test::ensure_file("cached_file.txt", Some("cached content"));
    
    // Add file to index
    let add_args = add::AddArgs {
        all: false,
        update: false,
        verbose: false,
        pathspec: vec!["cached_file.txt".to_string()],
        dry_run: false,
        refresh: false,
        ignore_errors: false,
    };
    add::execute(add_args).await;

    // Remove file from index only (keep in filesystem)
    let remove_args = remove::RemoveArgs {
        pathspec: vec!["cached_file.txt".to_string()],
        cached: true,
        recursive: false,
        force: false,
    };
    let _ = remove::execute(remove_args);

    // Verify file still exists in filesystem
    assert!(Path::new("cached_file.txt").exists());
}

async fn test_remove_directory() {
    println!("\n\x1b[1mTest remove directory functionality.\x1b[0m");

    // Create directory structure
    test::ensure_file("test_dir/file1.txt", Some("file1 content"));
    test::ensure_file("test_dir/file2.txt", Some("file2 content"));
    test::ensure_file("test_dir/subdir/file3.txt", Some("file3 content"));
    
    // Add all files to index
    let add_args = add::AddArgs {
        all: true,
        update: false,
        verbose: false,
        pathspec: vec![],
        dry_run: false,
        refresh: false,
        ignore_errors: false,
    };
    add::execute(add_args).await;

    // Remove directory recursively
    let remove_args = remove::RemoveArgs {
        pathspec: vec!["test_dir".to_string()],
        cached: false,
        recursive: true,
        force: false,
    };
    let _ = remove::execute(remove_args);

    // Verify directory is removed
    assert!(!Path::new("test_dir").exists());
}

async fn test_remove_directory_without_recursive() {
    println!("\n\x1b[1mTest remove directory without recursive flag.\x1b[0m");

    // Create directory structure
    test::ensure_file("test_dir2/file1.txt", Some("file1 content"));
    test::ensure_file("test_dir2/file2.txt", Some("file2 content"));
    
    // Add all files to index
    let add_args = add::AddArgs {
        all: true,
        update: false,
        verbose: false,
        pathspec: vec![],
        dry_run: false,
        refresh: false,
        ignore_errors: false,
    };
    add::execute(add_args).await;

    // Try to remove directory without recursive flag (should fail)
    let remove_args = remove::RemoveArgs {
        pathspec: vec!["test_dir2".to_string()],
        cached: false,
        recursive: false,
        force: false,
    };
    let _ = remove::execute(remove_args);

    // Directory should still exist because recursive flag was not used
    assert!(Path::new("test_dir2").exists());
}

async fn test_force_remove_untracked_file() {
    println!("\n\x1b[1mTest force remove untracked file.\x1b[0m");

    // Create an untracked file
    test::ensure_file("untracked_file.txt", Some("untracked content"));

    // Force remove untracked file
    let remove_args = remove::RemoveArgs {
        pathspec: vec!["untracked_file.txt".to_string()],
        cached: false,
        recursive: false,
        force: true,
    };
    let _ = remove::execute(remove_args);

    // Verify file is removed
    assert!(!Path::new("untracked_file.txt").exists());
}

async fn test_no_force_remove_untracked_file() {
    println!("\n\x1b[1mTest force remove untracked file.\x1b[0m");

    // Create an untracked file
    test::ensure_file("untracked_file.txt", Some("untracked content"));

    // Force remove untracked file
    let remove_args = remove::RemoveArgs {
        pathspec: vec!["untracked_file.txt".to_string()],
        cached: false,
        recursive: false,
        force: false,
    };
    let _ = remove::execute(remove_args);

    // Verify file is removed
    assert!(Path::new("untracked_file.txt").exists());
}

async fn test_force_remove_untracked_directory() {
    println!("\n\x1b[1mTest force remove untracked directory.\x1b[0m");

    // Create untracked directory structure
    test::ensure_file("untracked_dir/file1.txt", Some("file1 content"));
    test::ensure_file("untracked_dir/file2.txt", Some("file2 content"));
    test::ensure_file("untracked_dir/subdir/file3.txt", Some("file3 content"));

    // Force remove untracked directory
    let remove_args = remove::RemoveArgs {
        pathspec: vec!["untracked_dir".to_string()],
        cached: false,
        recursive: true,
        force: true,
    };
    let _ = remove::execute(remove_args);

    // Verify directory is removed
    assert!(!Path::new("untracked_dir").exists());
}

async fn test_remove_multiple_files() {
    println!("\n\x1b[1mTest remove multiple files.\x1b[0m");

    // Create multiple files
    test::ensure_file("file1.txt", Some("content1"));
    test::ensure_file("file2.txt", Some("content2"));
    test::ensure_file("file3.txt", Some("content3"));
    
    // Add files to index
    let add_args = add::AddArgs {
        all: true,
        update: false,
        verbose: false,
        pathspec: vec![],
        dry_run: false,
        refresh: false,
        ignore_errors: false,
    };
    add::execute(add_args).await;

    // Remove multiple files
    let remove_args = remove::RemoveArgs {
        pathspec: vec!["file1.txt".to_string(), "file2.txt".to_string(), "file3.txt".to_string()],
        cached: false,
        recursive: false,
        force: false,
    };
    let _ = remove::execute(remove_args);

    // Verify all files are removed
    assert!(!Path::new("file1.txt").exists());
    assert!(!Path::new("file2.txt").exists());
    assert!(!Path::new("file3.txt").exists());
}

async fn test_remove_nonexistent_file() {
    println!("\n\x1b[1mTest remove nonexistent file.\x1b[0m");

    // Try to remove a file that doesn't exist (should fail without force)
    let remove_args = remove::RemoveArgs {
        pathspec: vec!["nonexistent.txt".to_string()],
        cached: false,
        recursive: false,
        force: false,
    };
    let _ = remove::execute(remove_args);

    // With force flag, should not fail
    let remove_args_force = remove::RemoveArgs {
        pathspec: vec!["nonexistent.txt".to_string()],
        cached: false,
        recursive: false,
        force: true,
    };
    let _ = remove::execute(remove_args_force);
}

async fn test_remove_empty_pathspec() {
    println!("\n\x1b[1mTest remove with empty pathspec.\x1b[0m");

    // Try to remove with empty pathspec (should fail)
    let remove_args = remove::RemoveArgs {
        pathspec: vec![],
        cached: false,
        recursive: false,
        force: false,
    };
    let _ = remove::execute(remove_args);
}

#[tokio::test]
#[serial]
/// Tests the remove command functionality including:
/// - Basic file removal
/// - Cached removal (keep in filesystem)
/// - Directory removal with recursive flag
/// - Force removal of untracked files
/// - Multiple file removal
/// - Error handling for invalid paths
async fn test_remove_command() {
    println!("\n\x1b[1mTest remove command functionality.\x1b[0m");

    let temp_path = tempdir().unwrap();
    test::setup_clean_testing_env_in(temp_path.path());
    let _guard = test::ChangeDirGuard::new(temp_path.path());

    // Initialize repository
    let init_args = init::InitArgs {
        bare: false,
        initial_branch: Some("main".to_string()),
        repo_directory: temp_path.path().to_str().unwrap().to_string(),
        quiet: false,
    };
    init::init(init_args).await.expect("Error initializing repository");

    // Create initial commit
    let commit_args = commit::CommitArgs {
        message: "Initial commit".to_string(),
        allow_empty: true,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: true,
    };
    commit::execute(commit_args).await;

    // Run all tests
    test_remove_file().await;
    test_remove_cached_file().await;
    test_remove_directory().await;
    test_remove_directory_without_recursive().await;
    test_force_remove_untracked_file().await;
    test_force_remove_untracked_directory().await;
    test_no_force_remove_untracked_file().await;
    test_remove_multiple_files().await;
    test_remove_nonexistent_file().await;
    test_remove_empty_pathspec().await;

    println!("\n\x1b[32m✓ All remove command tests passed!\x1b[0m");
}
