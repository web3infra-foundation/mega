#![cfg(test)]
use super::*;
use libra::command::rebase::{execute, RebaseArgs};
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
#[serial]
async fn test_basic_rebase() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());

    // 1. Create initial commits on master
    fs::write(temp_path.path().join("file.txt"), "content1").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["file.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "C1: Add file.txt on master".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    fs::write(temp_path.path().join("file.txt"), "content1\ncontent2").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["file.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "C2: Modify file.txt on master".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    // 2. Create and switch to feature branch
    switch::execute(SwitchArgs {
        branch: None,
        create: Some("feature".to_string()),
        detach: false,
    })
    .await;

    // 3. Create commits on feature branch
    fs::write(temp_path.path().join("feature_a.txt"), "featureA").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["feature_a.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "F1: Add feature_a.txt on feature branch".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    fs::write(temp_path.path().join("feature_b.txt"), "featureB").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["feature_b.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "F2: Add feature_b.txt on feature branch".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    // 4. Switch back to master and make it diverge
    switch::execute(SwitchArgs {
        branch: Some("master".to_string()),
        create: None,
        detach: false,
    })
    .await;

    fs::write(temp_path.path().join("master_only.txt"), "master_change").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["master_only.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "C3: Add master_only.txt on master".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    // 5. Switch back to feature and perform rebase
    switch::execute(SwitchArgs {
        branch: Some("feature".to_string()),
        create: None,
        detach: false,
    })
    .await;

    execute(RebaseArgs {
        upstream: "master".to_string(),
    })
    .await;

    // 6. Verify the rebase result
    // Check that all files exist after rebase
    assert!(temp_path.path().join("file.txt").exists());
    assert!(temp_path.path().join("feature_a.txt").exists());
    assert!(temp_path.path().join("feature_b.txt").exists());
    assert!(temp_path.path().join("master_only.txt").exists());

    // Check file contents
    assert_eq!(
        fs::read_to_string(temp_path.path().join("file.txt")).unwrap(),
        "content1\ncontent2"
    );
    assert_eq!(
        fs::read_to_string(temp_path.path().join("feature_a.txt")).unwrap(),
        "featureA"
    );
    assert_eq!(
        fs::read_to_string(temp_path.path().join("feature_b.txt")).unwrap(),
        "featureB"
    );
    assert_eq!(
        fs::read_to_string(temp_path.path().join("master_only.txt")).unwrap(),
        "master_change"
    );

    println!("Basic rebase test passed successfully!");
}

#[tokio::test]
#[serial]
async fn test_rebase_already_up_to_date() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());

    // Create commits on master
    fs::write(temp_path.path().join("file1.txt"), "content1").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["file1.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "First commit".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    fs::write(temp_path.path().join("file2.txt"), "content2").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["file2.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "Second commit".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    // Create feature branch from current master (no divergence)
    switch::execute(SwitchArgs {
        branch: None,
        create: Some("feature".to_string()),
        detach: false,
    })
    .await;

    // Try to rebase feature onto master (should be up to date)
    execute(RebaseArgs {
        upstream: "master".to_string(),
    })
    .await;

    // Should complete without errors (already up to date)
    println!("Already up-to-date rebase test passed!");
}
