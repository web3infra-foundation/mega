use super::*;
use std::fs;
async fn test_check_status() {
    println!("\n\x1b[1mTest check_status function.\x1b[0m");

    // Test the check_status
    // Expect false when no changes
    assert!(!check_status().await);

    // Create a file and add it to the index
    // Expect true when there are unstaged changes
    fs::File::create("foo.txt").unwrap();
    let add_args = add::AddArgs {
        pathspec: vec!["foo.txt".to_string()],
        all: false,
        update: false,
        verbose: true,
    };
    add::execute(add_args).await;
    assert!(check_status().await);

    // Modify a file
    // Expect true when there are uncommitted changes
    fs::write("foo.txt", "modified content").unwrap();
    assert!(check_status().await);
}

async fn test_switch_function() {
    println!("\n\x1b[1mTest switch function.\x1b[0m");

    // create first empty commit
    {
        let args = CommitArgs {
            message: "first".to_string(),
            allow_empty: true,
            conventional: false,
            amend: false,
        };
        commit::execute(args).await;
    }

    // create a new branch and switch to it
    {
        let args = SwitchArgs {
            branch: None,
            create: Some("test_branch".to_string()),
            detach: false,
        };
        switch::execute(args).await;
        let head = Head::current().await;
        let ref_name = match head {
            Head::Branch(name) => name,
            _ => panic!("head not in branch,unreachable"),
            // Head::Detached(name) => name.to_string(),
        };
        assert_eq!(
            ref_name, "test_branch",
            "create a new branch and switch to it failed!"
        );
    }

    //detach the head to a commit
    {
        let head = Head::current().await;
        let ref_name = match head {
            Head::Branch(name) => name,
            _ => panic!("head not in branch,unreachable"),
            // Head::Detached(name) => name.to_string(),
        };
        let branch = Branch::find_branch(&ref_name, None).await.unwrap();
        let commit: Commit = load_object(&branch.commit).unwrap();
        let commit_id_str = commit.id.to_string();

        let args = CommitArgs {
            message: "second".to_string(),
            allow_empty: true,
            conventional: false,
            amend: false,
        };
        commit::execute(args).await;

        let args = SwitchArgs {
            branch: Some(commit_id_str.clone()),
            create: None,
            detach: true,
        };
        switch::execute(args).await;
        let head = Head::current().await;
        let ref_name = match head {
            Head::Detached(name) => name.to_string(),
            _ => panic!("head not detached,unreachable"),
            // Head::Detached(name) => name.to_string(),
        };
        println!("detach {:?}", ref_name);
        assert_eq!(
            ref_name, commit_id_str,
            "detach the head to a commit failed!"
        );
    }

    //switch branch back to the master
    {
        let args = SwitchArgs {
            branch: Some("master".to_string()),
            create: None,
            detach: false,
        };
        switch::execute(args).await;
        let head = Head::current().await;
        let ref_name = match head {
            Head::Branch(name) => name,
            _ => panic!("head not in branch,unreachable"),
            // Head::Detached(name) => name.to_string(),
        };
        assert_eq!(ref_name, "master", "switch bach to the master failed!");
    }
}
#[tokio::test]
#[serial]
/// Tests the core functionality of the switch command module.
/// Validates branch switching operations and working directory status checks.
async fn test_parts_of_switch_module_function() {
    println!("\n\x1b[1mTest some functions of the switch module.\x1b[0m");
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());
    println!("temp_path {:?}", temp_path);

    //Test check the branch
    test_switch_function().await;

    // Test the switch module funsctions
    test_check_status().await;
}
