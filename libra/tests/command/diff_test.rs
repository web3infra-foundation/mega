use super::*;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use libra::command::diff::{self, DiffArgs};

/// Helper function to create a file with content
fn create_file(path: &str, content: &str) {
    let mut file = fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

/// Helper function to modify a file with new content
fn modify_file(path: &str, content: &str) {
    let mut file = fs::OpenOptions::new().write(true).truncate(true).open(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

#[tokio::test]
#[serial]
/// Tests the basic diff functionality between working directory and HEAD
async fn test_basic_diff() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a file and add it to index
    create_file("file1.txt", "Initial content\nLine 2\nLine 3\n");
    
    add::execute(AddArgs {
        pathspec: vec![String::from("file1.txt")],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Create initial commit
    commit::execute(CommitArgs {
        message: Some("Initial commit".to_string()),
        allow_empty: false,
        all: false,
        amend: false,
        signoff: false,
    })
    .await
    .unwrap();

    // Modify the file
    modify_file("file1.txt", "Modified content\nLine 2\nLine 3 changed\n");

    // Run diff command
    diff::execute(DiffArgs {
        old: None,
        new: None,
        staged: false,
        pathspec: vec![],
        algorithm: Some("histogram".to_string()),
        output: None,
    })
    .await;

    // We can't easily capture stdout, so we'll check that the command didn't panic
}

#[tokio::test]
#[serial]
/// Tests diff with staged changes
async fn test_diff_staged() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a file and add it to index
    create_file("file1.txt", "Initial content\nLine 2\nLine 3\n");
    
    add::execute(AddArgs {
        pathspec: vec![String::from("file1.txt")],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Create initial commit
    commit::execute(CommitArgs {
        message: Some("Initial commit".to_string()),
        allow_empty: false,
        all: false,
        amend: false,
        signoff: false,
    })
    .await
    .unwrap();

    // Modify the file and stage it
    modify_file("file1.txt", "Modified content\nLine 2\nLine 3 changed\n");
    
    add::execute(AddArgs {
        pathspec: vec![String::from("file1.txt")],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Modify the file again (so working dir differs from staged)
    modify_file("file1.txt", "Modified content again\nLine 2\nLine 3 changed again\n");

    // Run diff command with --staged flag
    diff::execute(DiffArgs {
        old: None,
        new: None,
        staged: true,
        pathspec: vec![],
        algorithm: Some("histogram".to_string()),
        output: None,
    })
    .await;

    // The command should complete without panicking
}

#[tokio::test]
#[serial]
/// Tests diff between two specific commits
async fn test_diff_between_commits() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a file and make initial commit
    create_file("file1.txt", "Initial content\nLine 2\nLine 3\n");
    
    add::execute(AddArgs {
        pathspec: vec![String::from("file1.txt")],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    commit::execute(CommitArgs {
        message: Some("Initial commit".to_string()),
        allow_empty: false,
        all: false,
        amend: false,
        signoff: false,
    })
    .await
    .unwrap();

    // Get the first commit hash
    let first_commit = Head::current_commit().await.unwrap();

    // Modify file and create a second commit
    modify_file("file1.txt", "Modified content\nLine 2\nLine 3 changed\n");
    
    add::execute(AddArgs {
        pathspec: vec![String::from("file1.txt")],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    commit::execute(CommitArgs {
        message: Some("Second commit".to_string()),
        allow_empty: false,
        all: false,
        amend: false,
        signoff: false,
    })
    .await
    .unwrap();

    // Get the second commit hash
    let second_commit = Head::current_commit().await.unwrap();

    // Run diff command comparing the two commits
    diff::execute(DiffArgs {
        old: Some(first_commit.to_string()),
        new: Some(second_commit.to_string()),
        staged: false,
        pathspec: vec![],
        algorithm: Some("histogram".to_string()),
        output: None,
    })
    .await;

    // The command should complete without panicking
}

#[tokio::test]
#[serial]
/// Tests diff with specific file path
async fn test_diff_with_pathspec() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create multiple files and commit them
    create_file("file1.txt", "File 1 content\nLine 2\nLine 3\n");
    create_file("file2.txt", "File 2 content\nLine 2\nLine 3\n");
    
    add::execute(AddArgs {
        pathspec: vec![String::from(".")]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    commit::execute(CommitArgs {
        message: Some("Initial commit".to_string()),
        allow_empty: false,
        all: false,
        amend: false,
        signoff: false,
    })
    .await
    .unwrap();

    // Modify both files
    modify_file("file1.txt", "File 1 modified\nLine 2\nLine 3 changed\n");
    modify_file("file2.txt", "File 2 modified\nLine 2\nLine 3 changed\n");

    // Run diff command with specific file path
    diff::execute(DiffArgs {
        old: None,
        new: None,
        staged: false,
        pathspec: vec![String::from("file1.txt")],
        algorithm: Some("histogram".to_string()),
        output: None,
    })
    .await;

    // The command should complete without panicking
}

#[tokio::test]
#[serial]
/// Tests diff with output to a file
async fn test_diff_output_to_file() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a file and commit it
    create_file("file1.txt", "Initial content\nLine 2\nLine 3\n");
    
    add::execute(AddArgs {
        pathspec: vec![String::from("file1.txt")],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    commit::execute(CommitArgs {
        message: Some("Initial commit".to_string()),
        allow_empty: false,
        all: false,
        amend: false,
        signoff: false,
    })
    .await
    .unwrap();

    // Modify the file
    modify_file("file1.txt", "Modified content\nLine 2\nLine 3 changed\n");

    // Output file path
    let output_file = "diff_output.txt";

    // Run diff command with output to file
    diff::execute(DiffArgs {
        old: None,
        new: None,
        staged: false,
        pathspec: vec![],
        algorithm: Some("histogram".to_string()),
        output: Some(output_file.to_string()),
    })
    .await;

    // Verify the output file exists
    assert!(fs::metadata(output_file).is_ok(), "Output file should exist");
    
    // Read the file content to make sure it contains diff output
    let content = fs::read_to_string(output_file).unwrap();
    assert!(content.contains("diff --git"), "Output should contain diff header");
}

#[tokio::test]
#[serial]
/// Tests diff with different algorithms
async fn test_diff_algorithms() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a file with some content to make a non-trivial diff
    create_file(
        "file1.txt",
        "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\n",
    );
    
    add::execute(AddArgs {
        pathspec: vec![String::from("file1.txt")],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    commit::execute(CommitArgs {
        message: Some("Initial commit".to_string()),
        allow_empty: false,
        all: false,
        amend: false,
        signoff: false,
    })
    .await
    .unwrap();

    // Make complex changes to test different algorithms
    modify_file(
        "file1.txt",
        "Line 1\nModified Line\nLine 3\nNew Line\nLine 5\nLine 6\nDeleted Line 7\n",
    );

    // Test histogram algorithm
    diff::execute(DiffArgs {
        old: None,
        new: None,
        staged: false,
        pathspec: vec![],
        algorithm: Some("histogram".to_string()),
        output: Some("histogram_diff.txt".to_string()),
    })
    .await;

    // Test myers algorithm
    diff::execute(DiffArgs {
        old: None,
        new: None,
        staged: false,
        pathspec: vec![],
        algorithm: Some("myers".to_string()),
        output: Some("myers_diff.txt".to_string()),
    })
    .await;

    // Test myersMinimal algorithm
    diff::execute(DiffArgs {
        old: None,
        new: None,
        staged: false,
        pathspec: vec![],
        algorithm: Some("myersMinimal".to_string()),
        output: Some("myersMinimal_diff.txt".to_string()),
    })
    .await;

    // Verify all output files exist
    assert!(fs::metadata("histogram_diff.txt").is_ok(), "Histogram output file should exist");
    assert!(fs::metadata("myers_diff.txt").is_ok(), "Myers output file should exist");
    assert!(fs::metadata("myersMinimal_diff.txt").is_ok(), "MyersMinimal output file should exist");
}
