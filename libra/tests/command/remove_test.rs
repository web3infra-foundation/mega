use super::*;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use libra::command::remove::{self, RemoveArgs};

/// Helper function to create a file with content
fn create_file(path: &str, content: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    path
}

#[tokio::test]
#[serial]
/// Tests the basic remove functionality by removing a single file
async fn test_remove_single_file() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a file and add it to index
    let file_path = create_file("test_file.txt", "Test content");
    
    add::execute(AddArgs {
        pathspec: vec![String::from("test_file.txt")],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Make sure the file exists
    assert!(file_path.exists(), "File should exist before removal");

    // Remove the file
    let args = RemoveArgs::try_parse_from([
        "remove", "test_file.txt"
    ]).unwrap();
    remove::execute(args).unwrap();

    // Verify the file was removed from the filesystem
    assert!(!file_path.exists(), "File should be removed from filesystem");

    // Verify file is no longer in the index
    let changes = changes_to_be_staged();
    assert!(!changes.new.iter().any(|x| x.to_str().unwrap() == "test_file.txt"), 
        "File should not appear in changes as new");
    assert!(!changes.modified.iter().any(|x| x.to_str().unwrap() == "test_file.txt"), 
        "File should not appear in changes as modified");
    assert!(!changes.deleted.iter().any(|x| x.to_str().unwrap() == "test_file.txt"), 
        "File should not appear in changes as deleted");
}

#[tokio::test]
#[serial]
/// Tests removing a file with --cached flag, which only removes from the index but keeps the file
async fn test_remove_cached() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a file and add it to index
    let file_path = create_file("test_file.txt", "Test content");
    
    add::execute(AddArgs {
        pathspec: vec![String::from("test_file.txt")],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Make sure the file exists
    assert!(file_path.exists(), "File should exist before removal");

    // Remove the file with --cached flag
    let args = RemoveArgs::try_parse_from([
        "remove", "--cached", "test_file.txt"
    ]).unwrap();
    remove::execute(args).unwrap();

    // Verify the file still exists in the filesystem
    assert!(file_path.exists(), "File should still exist in filesystem");

    // Verify file appears as new (untracked) in the index
    let changes = changes_to_be_staged();
    assert!(changes.new.iter().any(|x| x.to_str().unwrap() == "test_file.txt"), 
        "File should appear in changes as new/untracked");
}

#[tokio::test]
#[serial]
/// Tests recursive removal of a directory
async fn test_remove_directory_recursive() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a directory with files
    let file1 = create_file("test_dir/file1.txt", "File 1 content");
    let file2 = create_file("test_dir/file2.txt", "File 2 content");
    let file3 = create_file("test_dir/subdir/file3.txt", "File 3 content");
    
    // Add all files to the index
    add::execute(AddArgs {
        pathspec: vec![String::from("test_dir")],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Make sure the directory and files exist
    assert!(fs::metadata("test_dir").is_ok(), "Directory should exist");
    assert!(file1.exists(), "File 1 should exist");
    assert!(file2.exists(), "File 2 should exist");
    assert!(file3.exists(), "File 3 should exist");

    // Remove the directory recursively
    let args = RemoveArgs::try_parse_from([
        "remove", "--recursive", "test_dir"
    ]).unwrap();
    remove::execute(args).unwrap();

    // Verify the directory and files were removed
    assert!(fs::metadata("test_dir").is_err(), "Directory should be removed");
    assert!(!file1.exists(), "File 1 should be removed");
    assert!(!file2.exists(), "File 2 should be removed");
    assert!(!file3.exists(), "File 3 should be removed");

    // Verify files are no longer in the index
    let changes = changes_to_be_staged();
    for file in &[file1, file2, file3] {
        let file_str = file.to_str().unwrap();
        assert!(!changes.new.iter().any(|x| x.to_str().unwrap() == file_str), 
            "File should not appear in changes as new");
        assert!(!changes.modified.iter().any(|x| x.to_str().unwrap() == file_str), 
            "File should not appear in changes as modified");
        assert!(!changes.deleted.iter().any(|x| x.to_str().unwrap() == file_str), 
            "File should not appear in changes as deleted");
    }
}

#[tokio::test]
#[serial]
/// Tests attempting to remove a directory without -r flag should fail
async fn test_remove_directory_without_recursive() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a directory with files
    let file1 = create_file("test_dir/file1.txt", "File 1 content");
    let file2 = create_file("test_dir/file2.txt", "File 2 content");
    
    // Add all files to the index
    add::execute(AddArgs {
        pathspec: vec![String::from("test_dir")],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Make sure the directory and files exist
    assert!(fs::metadata("test_dir").is_ok(), "Directory should exist");
    assert!(file1.exists(), "File 1 should exist");
    assert!(file2.exists(), "File 2 should exist");

    // Attempt to remove the directory without recursive flag
    let args = RemoveArgs::try_parse_from([
        "remove", "test_dir"
    ]).unwrap();
    remove::execute(args).unwrap(); // This should not error, but it should not remove anything either

    // Verify the directory and files still exist
    assert!(fs::metadata("test_dir").is_ok(), "Directory should still exist");
    assert!(file1.exists(), "File 1 should still exist");
    assert!(file2.exists(), "File 2 should still exist");
}

#[tokio::test]
#[serial]
/// Tests removing a file that does not exist in the index
async fn test_remove_untracked_file() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a file but don't add it to the index
    let file_path = create_file("untracked_file.txt", "Untracked content");
    
    // Make sure the file exists
    assert!(file_path.exists(), "File should exist");

    // Attempt to remove the untracked file (should fail/do nothing)
    let args = RemoveArgs::try_parse_from([
        "remove", "untracked_file.txt"
    ]).unwrap();
    remove::execute(args).unwrap(); // Should not panic but should print error

    // Verify the file still exists
    assert!(file_path.exists(), "File should still exist");
}

#[tokio::test]
#[serial]
/// Tests removing a file that has been modified after being added to the index
async fn test_remove_modified_file() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a file and add it to index
    let file_path = create_file("test_file.txt", "Initial content");
    
    add::execute(AddArgs {
        pathspec: vec![String::from("test_file.txt")],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Modify the file
    let mut file = fs::OpenOptions::new().write(true).truncate(true).open(&file_path).unwrap();
    file.write_all(b" - Modified").unwrap();

    // Remove the file
    let args = RemoveArgs::try_parse_from([
        "remove", "test_file.txt"
    ]).unwrap();
    remove::execute(args).unwrap();

    // Verify the file was removed
    assert!(!file_path.exists(), "File should be removed");

    // Verify file is not in the index
    let changes = changes_to_be_staged();
    assert!(!changes.new.iter().any(|x| x.to_str().unwrap() == "test_file.txt"), 
        "File should not appear in changes as new");
    assert!(!changes.modified.iter().any(|x| x.to_str().unwrap() == "test_file.txt"), 
        "File should not appear in changes as modified");
    assert!(!changes.deleted.iter().any(|x| x.to_str().unwrap() == "test_file.txt"), 
        "File should not appear in changes as deleted");
}

#[tokio::test]
#[serial]
/// Tests removing multiple files at once
async fn test_remove_multiple_files() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create multiple files
    let file1 = create_file("file1.txt", "File 1 content");
    let file2 = create_file("file2.txt", "File 2 content");
    let file3 = create_file("file3.txt", "File 3 content");
    
    // Add all files to the index
    add::execute(AddArgs {
        pathspec: vec![String::from(".")],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Make sure all files exist
    assert!(file1.exists(), "File 1 should exist");
    assert!(file2.exists(), "File 2 should exist");
    assert!(file3.exists(), "File 3 should exist");

    // Remove multiple files at once
    let args = RemoveArgs::try_parse_from([
        "remove", "file1.txt", "file3.txt"
    ]).unwrap();
    remove::execute(args).unwrap();

    // Verify the specified files were removed
    assert!(!file1.exists(), "File 1 should be removed");
    assert!(file2.exists(), "File 2 should still exist");
    assert!(!file3.exists(), "File 3 should be removed");
}
