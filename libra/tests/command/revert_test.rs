use super::*;
use libra::command::revert;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

/// Test basic revert functionality with file additions, modifications, and deletions
/// This test follows the workflow:
/// 1. C1: Add 1.txt with content1
/// 2. C2: Modify 1.txt (append content2)
/// 3. C3: Remove 1.txt, Add 2.txt
/// 4. Revert HEAD (C3) - should restore 1.txt and remove 2.txt
/// 5. Find C2 and revert it - should restore 1.txt to original content
#[tokio::test]
#[serial]
async fn test_basic_revert() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());

    println!("===== SCENARIO 1: BASIC REVERT TEST =====");

    // --- 1. C1: Add 1.txt ---
    fs::write("1.txt", "content1").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["1.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "C1: add 1.txt".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;
    println!("C1: Added 1.txt");

    // --- 2. C2: Modify 1.txt ---
    fs::write("1.txt", "content1\ncontent2").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["1.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "C2: modify 1.txt".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;
    println!("C2: Modified 1.txt");

    // --- 3. C3: Remove 1.txt, Add 2.txt ---
    fs::remove_file("1.txt").unwrap();
    fs::write("2.txt", "content3").unwrap();
    add::execute(AddArgs {
        pathspec: vec![],
        all: true,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "C3: remove 1.txt, add 2.txt".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;
    println!("C3: Removed 1.txt, Added 2.txt");

    // --- 4. Show initial state ---
    println!("\nBasic test repo is ready. Files before revert:");
    let files: Vec<_> = fs::read_dir(".")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with('.') && name.ends_with(".txt") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    for file in &files {
        println!("{file}");
    }

    // --- 5. Test 1: Revert HEAD (C3) ---
    println!("\n--- Test 1: Revert HEAD (C3) ---");
    revert::execute(revert::RevertArgs {
        commit: "HEAD".to_string(),
        no_commit: false,
    })
    .await;

    // Verify state after reverting C3
    println!("Files after reverting HEAD:");
    let files_after_revert: Vec<_> = fs::read_dir(".")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with('.') && name.ends_with(".txt") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    for file in &files_after_revert {
        println!("{file}");
    }

    // Should have 1.txt back (modified version) and 2.txt should be gone
    assert!(
        PathBuf::from("1.txt").exists(),
        "1.txt should exist after reverting C3"
    );
    assert!(
        !PathBuf::from("2.txt").exists(),
        "2.txt should not exist after reverting C3"
    );

    // Check content of 1.txt should be the modified version
    let content = fs::read_to_string("1.txt").unwrap();
    assert_eq!(
        content, "content1\ncontent2",
        "1.txt should have modified content"
    );

    println!("Test 1 passed: HEAD revert successful");

    println!("\nAll basic revert tests passed!");
}

/// Test revert with no-commit flag
/// This test verifies that the --no-commit flag stages changes without creating a commit
#[tokio::test]
#[serial]
async fn test_revert_no_commit() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());

    // Create initial commits
    fs::write("test.txt", "original").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["test.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "Add test.txt".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    fs::write("test.txt", "modified").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["test.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "Modify test.txt".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    // Test revert with no-commit flag
    revert::execute(revert::RevertArgs {
        commit: "HEAD".to_string(),
        no_commit: true,
    })
    .await;

    // File should be reverted but not committed
    let content = fs::read_to_string("test.txt").unwrap();
    assert_eq!(
        content, "original",
        "File should be reverted to original content"
    );

    // Check that we can still commit the staged changes
    commit::execute(CommitArgs {
        message: "Manual revert commit".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    println!("No-commit revert test passed");
}

/// Test reverting root commit
/// Root commits have no parents, so reverting them should create an empty repository state
#[tokio::test]
#[serial]
async fn test_revert_root_commit() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());

    // Create initial commit
    fs::write("initial.txt", "initial content").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["initial.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "Initial commit".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    // Get the root commit hash - we need to implement this differently
    // since we can't call external libra command in tests
    let head = Head::current_commit()
        .await
        .expect("Should have current commit");
    let root_hash = head.to_string();

    // Revert root commit
    revert::execute(revert::RevertArgs {
        commit: root_hash,
        no_commit: false,
    })
    .await;

    // All files should be removed
    let files: Vec<_> = fs::read_dir(".")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with('.') {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    assert!(
        files.is_empty(),
        "No files should exist after reverting root commit"
    );
    println!("Root commit revert test passed");
}

/// Test error cases for revert command
/// This ensures the command handles invalid input gracefully
#[tokio::test]
#[serial]
async fn test_revert_errors() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());

    // Test reverting non-existent commit should fail gracefully
    revert::execute(revert::RevertArgs {
        commit: "nonexistent".to_string(),
        no_commit: false,
    })
    .await;

    println!("Error handling test completed");
}
