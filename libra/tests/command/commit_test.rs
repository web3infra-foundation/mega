use serial_test::serial;
use tempfile::tempdir;

use super::*;
#[tokio::test]
#[serial]
#[should_panic]
/// A commit with no file changes should fail if `allow_empty` is false.
/// This test verifies that the commit command rejects empty changesets
/// when not explicitly permitted.
async fn test_execute_commit_with_empty_index_fail() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());

    let args = CommitArgs {
        message: "init".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: true,
    };
    commit::execute(args).await;
}

#[tokio::test]
#[serial]
/// Tests normal commit functionality with both `--amend` and `--allow_empty` flags.
/// Verifies that:
/// 1. Amending works correctly when allowed
/// 2. Empty commits are permitted when explicitly enabled
async fn test_execute_commit() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());
    // create first empty commit
    {
        let args = CommitArgs {
            message: "init".to_string(),
            allow_empty: true,
            conventional: false,
            amend: false,
            signoff: false,
            disable_pre: true,
        };
        commit::execute(args).await;

        // check head branch exists
        let head = Head::current().await;
        let branch_name = match head {
            Head::Branch(name) => name,
            _ => panic!("head not in branch"),
        };
        let branch = Branch::find_branch(&branch_name, None).await.unwrap();
        let commit: Commit = load_object(&branch.commit).unwrap();

        assert_eq!(commit.message.trim(), "init");
        let branch = Branch::find_branch(&branch_name, None).await.unwrap();
        assert_eq!(branch.commit, commit.id);
    }

    // modify first empty commit
    {
        let args = CommitArgs {
            message: "init commit".to_string(),
            allow_empty: true,
            conventional: false,
            amend: true,
            signoff: false,
            disable_pre: true,
        };
        commit::execute(args).await;

        // check head branch exists
        let head = Head::current().await;
        let branch_name = match head {
            Head::Branch(name) => name,
            _ => panic!("head not in branch"),
        };
        let branch = Branch::find_branch(&branch_name, None).await.unwrap();
        let commit: Commit = load_object(&branch.commit).unwrap();

        assert_eq!(commit.message.trim(), "init commit");
        let branch = Branch::find_branch(&branch_name, None).await.unwrap();
        assert_eq!(branch.commit, commit.id);
    }

    // create a new commit
    {
        // create `a.txt` `bb/b.txt` `bb/c.txt`
        test::ensure_file("a.txt", Some("a"));
        test::ensure_file("bb/b.txt", Some("b"));
        test::ensure_file("bb/c.txt", Some("c"));
        let args = AddArgs {
            all: true,
            update: false,
            verbose: false,
            pathspec: vec![],
            dry_run: false,
            ignore_errors: false,
        };
        add::execute(args).await;
    }

    {
        let args = CommitArgs {
            message: "add some files".to_string(),
            allow_empty: false,
            conventional: false,
            amend: false,
            signoff: false,
            disable_pre: true,
        };
        commit::execute(args).await;

        let commit_id = Head::current_commit().await.unwrap();
        let commit: Commit = load_object(&commit_id).unwrap();
        assert_eq!(
            commit.message.trim(),
            "add some files",
            "{}",
            commit.message
        );

        let pre_commit_id = commit.parent_commit_ids[0];
        let pre_commit: Commit = load_object(&pre_commit_id).unwrap();
        assert_eq!(pre_commit.message.trim(), "init commit");

        let tree_id = commit.tree_id;
        let tree: Tree = load_object(&tree_id).unwrap();
        assert_eq!(tree.tree_items.len(), 2); // 2 subtree according to the test data
    }
    //modify new commit
    {
        let args = CommitArgs {
            message: "add some txt files".to_string(),
            allow_empty: true,
            conventional: false,
            amend: true,
            signoff: false,
            disable_pre: true,
        };
        commit::execute(args).await;

        let commit_id = Head::current_commit().await.unwrap();
        let commit: Commit = load_object(&commit_id).unwrap();
        assert_eq!(
            commit.message.trim(),
            "add some txt files",
            "{}",
            commit.message
        );

        let pre_commit_id = commit.parent_commit_ids[0];
        let pre_commit: Commit = load_object(&pre_commit_id).unwrap();
        assert_eq!(pre_commit.message.trim(), "init commit");

        let tree_id = commit.tree_id;
        let tree: Tree = load_object(&tree_id).unwrap();
        assert_eq!(tree.tree_items.len(), 2); // 2 subtree according to the test data
    }
}
