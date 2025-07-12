#![cfg(test)]
use super::*;
use serial_test::serial;
use tempfile::tempdir;
#[tokio::test]
#[serial]
/// Tests core branch management functionality including creation and listing.
/// Verifies branches can be created from specific commits.
async fn test_branch() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());

    let commit_args = CommitArgs {
        message: "first".to_string(),
        allow_empty: true,
        conventional: false,
        amend: false,
        signoff: false,
    };
    commit::execute(commit_args).await;
    let first_commit_id = Branch::find_branch("master", None).await.unwrap().commit;

    let commit_args = CommitArgs {
        message: "second".to_string(),
        allow_empty: true,
        conventional: false,
        amend: false,
        signoff: false,
    };
    commit::execute(commit_args).await;
    let second_commit_id = Branch::find_branch("master", None).await.unwrap().commit;

    {
        // create branch with first commit
        let first_branch_name = "first_branch".to_string();
        let args = BranchArgs {
            new_branch: Some(first_branch_name.clone()),
            commit_hash: Some(first_commit_id.to_string()),
            list: false,
            delete: None,
            set_upstream_to: None,
            show_current: false,
            remotes: false,
        };
        execute(args).await;

        // check branch exist
        match Head::current().await {
            Head::Branch(current_branch) => {
                assert_ne!(current_branch, first_branch_name)
            }
            _ => panic!("should be branch"),
        };

        let first_branch = Branch::find_branch(&first_branch_name, None).await.unwrap();
        assert_eq!(first_branch.commit, first_commit_id);
        assert_eq!(first_branch.name, first_branch_name);
    }

    {
        // create second branch with current branch
        let second_branch_name = "second_branch".to_string();
        let args = BranchArgs {
            new_branch: Some(second_branch_name.clone()),
            commit_hash: None,
            list: false,
            delete: None,
            set_upstream_to: None,
            show_current: false,
            remotes: false,
        };
        execute(args).await;
        let second_branch = Branch::find_branch(&second_branch_name, None)
            .await
            .unwrap();
        assert_eq!(second_branch.commit, second_commit_id);
        assert_eq!(second_branch.name, second_branch_name);
    }

    // show current branch
    println!("show current branch");
    let args = BranchArgs {
        new_branch: None,
        commit_hash: None,
        list: false,
        delete: None,
        set_upstream_to: None,
        show_current: true,
        remotes: false,
    };
    execute(args).await;

    // list branches
    println!("list branches");
    // execute(BranchArgs::parse_from([""])).await; // default list
}

#[tokio::test]
#[serial]
/// Tests branch creation using remote branches as starting points.
/// Verifies that local branches can be created from remote branch references.
async fn test_create_branch_from_remote() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());
    test::init_debug_logger();

    let args = CommitArgs {
        message: "first".to_string(),
        allow_empty: true,
        conventional: false,
        amend: false,
        signoff: false,
    };
    commit::execute(args).await;
    let hash = Head::current_commit().await.unwrap();
    Branch::update_branch("master", &hash.to_string(), Some("origin")).await; // create remote branch
    assert!(get_target_commit("origin/master").await.is_ok());

    let args = BranchArgs {
        new_branch: Some("test_new".to_string()),
        commit_hash: Some("origin/master".into()),
        list: false,
        delete: None,
        set_upstream_to: None,
        show_current: false,
        remotes: false,
    };
    execute(args).await;

    let branch = Branch::find_branch("test_new", None)
        .await
        .expect("branch create failed found");
    assert_eq!(branch.commit, hash);
}

#[tokio::test]
#[serial]
/// Tests the behavior of creating a branch with an invalid name.
async fn test_invalid_branch_name() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());
    test::init_debug_logger();

    let args = CommitArgs {
        message: "first".to_string(),
        allow_empty: true,
        conventional: false,
        amend: false,
        signoff: false,
    };
    commit::execute(args).await;

    let args = BranchArgs {
        new_branch: Some("@{mega}".to_string()),
        commit_hash: None,
        list: false,
        delete: None,
        set_upstream_to: None,
        show_current: false,
        remotes: false,
    };
    execute(args).await;

    let branch = Branch::find_branch("@{mega}", None).await;
    assert!(branch.is_none(), "invalid branch should not be created");
}
