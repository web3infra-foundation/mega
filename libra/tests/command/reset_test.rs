use super::*;
use libra::command::branch::{self, BranchArgs};
use libra::command::reset::{self, ResetArgs};
use libra::command::status::changes_to_be_staged;
use std::fs;

/// Setup a standard test repository with 4 commits and branches
async fn setup_standard_repo(temp_path: &std::path::Path) -> (SHA1, SHA1, SHA1, SHA1) {
    test::setup_with_new_libra_in(temp_path).await;

    fs::write("1.txt", "content 1").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["1.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
        refresh: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "commit 1: add 1.txt".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: true,
    })
    .await;
    let commit1 = Head::current_commit().await.unwrap();
    branch::execute(BranchArgs {
        new_branch: Some("1".to_string()),
        commit_hash: None,
        list: false,
        delete: None,
        set_upstream_to: None,
        show_current: false,
        remotes: false,
    })
    .await;

    fs::write("2.txt", "content 2").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["2.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
        refresh: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "commit 2: add 2.txt".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: true,
    })
    .await;
    let commit2 = Head::current_commit().await.unwrap();
    branch::execute(BranchArgs {
        new_branch: Some("2".to_string()),
        commit_hash: None,
        list: false,
        delete: None,
        set_upstream_to: None,
        show_current: false,
        remotes: false,
    })
    .await;

    fs::write("3.txt", "content 3").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["3.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
        refresh: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "commit 3: add 3.txt".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: true,
    })
    .await;
    let commit3 = Head::current_commit().await.unwrap();
    branch::execute(BranchArgs {
        new_branch: Some("3".to_string()),
        commit_hash: None,
        list: false,
        delete: None,
        set_upstream_to: None,
        show_current: false,
        remotes: false,
    })
    .await;

    fs::write("4.txt", "content 4").unwrap();
    add::execute(AddArgs {
        pathspec: vec!["4.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
        refresh: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "commit 4: add 4.txt".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: true,
    })
    .await;
    let commit4 = Head::current_commit().await.unwrap();
    branch::execute(BranchArgs {
        new_branch: Some("4".to_string()),
        commit_hash: None,
        list: false,
        delete: None,
        set_upstream_to: None,
        show_current: false,
        remotes: false,
    })
    .await;

    (commit1, commit2, commit3, commit4)
}

/// Setup the standard test state: modify files and stage some changes
async fn setup_test_state() {
    fs::write("3.txt", "content 3\nnew line").unwrap();
    fs::write("4.txt", "content 4\nnew line").unwrap();

    fs::write("5.txt", "new line").unwrap();

    add::execute(AddArgs {
        pathspec: vec!["3.txt".to_string()],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
        refresh: false,
    })
    .await;
}

#[tokio::test]
#[serial]
/// Tests soft reset: only moves HEAD pointer, preserves index and working directory
async fn test_reset_soft() {
    let temp_path = tempdir().unwrap();
    let _guard = ChangeDirGuard::new(temp_path.path());

    let (commit1, _, _, _) = setup_standard_repo(temp_path.path()).await;
    setup_test_state().await;

    // Perform soft reset to commit 1
    reset::execute(ResetArgs {
        target: "1".to_string(), // Reset to branch 1
        soft: true,
        mixed: false,
        hard: false,
        pathspecs: vec![],
    })
    .await;

    // Verify HEAD moved to commit 1
    let current_commit = Head::current_commit().await.unwrap();
    assert_eq!(current_commit, commit1);

    // Verify all files still exist in working directory
    assert!(fs::metadata("1.txt").is_ok());
    assert!(fs::metadata("2.txt").is_ok());
    assert!(fs::metadata("3.txt").is_ok());
    assert!(fs::metadata("4.txt").is_ok());
    assert!(fs::metadata("5.txt").is_ok());

    // Verify file contents are preserved (including modifications)
    assert_eq!(fs::read_to_string("3.txt").unwrap(), "content 3\nnew line");
    assert_eq!(fs::read_to_string("4.txt").unwrap(), "content 4\nnew line");
    assert_eq!(fs::read_to_string("5.txt").unwrap(), "new line");

    // Verify index still has staged changes (3.txt should be staged)
    let staged = libra::command::status::changes_to_be_committed().await;
    assert!(
        !staged.is_empty(),
        "Staged changes should be preserved in soft reset"
    );
}

#[tokio::test]
#[serial]
/// Tests mixed reset: moves HEAD and resets index, preserves working directory
async fn test_reset_mixed() {
    let temp_path = tempdir().unwrap();
    let _guard = ChangeDirGuard::new(temp_path.path());

    let (commit1, _, _, _) = setup_standard_repo(temp_path.path()).await;
    setup_test_state().await;

    // Perform mixed reset (default) to commit 1
    reset::execute(ResetArgs {
        target: "1".to_string(), // Reset to branch 1
        soft: false,
        mixed: false, // false means default (mixed)
        hard: false,
        pathspecs: vec![],
    })
    .await;

    // Verify HEAD moved to commit 1
    let current_commit = Head::current_commit().await.unwrap();
    assert_eq!(current_commit, commit1);

    // Verify all files still exist in working directory
    assert!(fs::metadata("1.txt").is_ok());
    assert!(fs::metadata("2.txt").is_ok());
    assert!(fs::metadata("3.txt").is_ok());
    assert!(fs::metadata("4.txt").is_ok());
    assert!(fs::metadata("5.txt").is_ok());

    // Verify file contents are preserved (including modifications)
    assert_eq!(fs::read_to_string("3.txt").unwrap(), "content 3\nnew line");
    assert_eq!(fs::read_to_string("4.txt").unwrap(), "content 4\nnew line");
    assert_eq!(fs::read_to_string("5.txt").unwrap(), "new line");

    // Verify index was reset (no staged changes)
    let staged = libra::command::status::changes_to_be_committed().await;
    assert!(staged.is_empty(), "Index should be reset in mixed reset");

    // Verify unstaged changes exist (2.txt, 3.txt, 4.txt should be untracked/modified)
    let unstaged = changes_to_be_staged();
    assert!(
        !unstaged.new.is_empty() || !unstaged.modified.is_empty(),
        "Should have unstaged changes after mixed reset"
    );
}

#[tokio::test]
#[serial]
/// Tests hard reset: moves HEAD, resets index and working directory
async fn test_reset_hard() {
    let temp_path = tempdir().unwrap();
    let _guard = ChangeDirGuard::new(temp_path.path());

    let (commit1, _, _, _) = setup_standard_repo(temp_path.path()).await;
    setup_test_state().await;

    // Perform hard reset to commit 1
    reset::execute(ResetArgs {
        target: "1".to_string(), // Reset to branch 1
        soft: false,
        mixed: false,
        hard: true,
        pathspecs: vec![],
    })
    .await;

    // Verify HEAD moved to commit 1
    let current_commit = Head::current_commit().await.unwrap();
    assert_eq!(current_commit, commit1);

    // Verify working directory was reset - only 1.txt should exist from commit 1
    assert!(fs::metadata("1.txt").is_ok());
    assert!(
        fs::metadata("2.txt").is_err(),
        "2.txt should be removed by hard reset"
    );
    assert!(
        fs::metadata("3.txt").is_err(),
        "3.txt should be removed by hard reset"
    );
    assert!(
        fs::metadata("4.txt").is_err(),
        "4.txt should be removed by hard reset"
    );

    // Untracked files should remain
    assert!(
        fs::metadata("5.txt").is_ok(),
        "Untracked files should remain after hard reset"
    );

    // Verify file content was restored to commit 1 state
    assert_eq!(fs::read_to_string("1.txt").unwrap(), "content 1");
    assert_eq!(fs::read_to_string("5.txt").unwrap(), "new line");

    // Verify index was reset
    let staged = libra::command::status::changes_to_be_committed().await;
    assert!(staged.is_empty(), "Index should be reset in hard reset");

    // Verify only untracked files remain
    let unstaged = changes_to_be_staged();
    assert!(
        !unstaged.new.is_empty(),
        "Should have untracked files (5.txt)"
    );
    assert!(
        unstaged.modified.is_empty(),
        "Should have no modified files"
    );
    assert!(unstaged.deleted.is_empty(), "Should have no deleted files");
}

#[tokio::test]
#[serial]
/// Tests reset with HEAD~ syntax
async fn test_reset_with_head_reference() {
    let temp_path = tempdir().unwrap();
    let _guard = ChangeDirGuard::new(temp_path.path());

    let (_, _, _, _) = setup_standard_repo(temp_path.path()).await;
    let second_commit = Head::current_commit().await.unwrap();

    // Reset using HEAD~ syntax
    reset::execute(ResetArgs {
        target: "HEAD~1".to_string(),
        soft: false,
        mixed: true,
        hard: false,
        pathspecs: vec![],
    })
    .await;

    // Verify HEAD moved back one commit
    let current_commit = Head::current_commit().await.unwrap();
    assert_ne!(current_commit, second_commit);

    // Verify working directory still has files
    assert!(fs::metadata("1.txt").is_ok());
    assert!(fs::metadata("4.txt").is_ok());

    // Verify index was reset (4.txt should be untracked)
    let unstaged = changes_to_be_staged();
    assert!(
        unstaged
            .new
            .iter()
            .any(|path| path.file_name().unwrap() == "4.txt")
    );
}

#[tokio::test]
#[serial]
/// Tests reset on a branch (should move branch pointer, not create detached HEAD)
async fn test_reset_on_branch() {
    let temp_path = tempdir().unwrap();
    let _guard = ChangeDirGuard::new(temp_path.path());

    let (commit1, _, _, _) = setup_standard_repo(temp_path.path()).await;

    // Verify we're on a branch before reset
    let head_before = Head::current().await;
    match head_before {
        Head::Branch(branch_name) => {
            assert_eq!(branch_name, "master"); // Default branch name

            // Perform reset
            reset::execute(ResetArgs {
                target: commit1.to_string(),
                soft: true,
                mixed: false,
                hard: false,
                pathspecs: vec![],
            })
            .await;

            // Verify we're still on the same branch after reset
            let head_after = Head::current().await;
            match head_after {
                Head::Branch(branch_name_after) => {
                    assert_eq!(branch_name_after, branch_name);
                }
                Head::Detached(_) => {
                    panic!("Reset should not create detached HEAD when on a branch");
                }
            }

            // Verify the branch pointer moved
            let current_commit = Head::current_commit().await.unwrap();
            assert_eq!(current_commit, commit1);
        }
        Head::Detached(_) => {
            panic!("Should be on a branch initially");
        }
    }
}
