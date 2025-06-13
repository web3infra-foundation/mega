use libra::utils::client_storage::ClientStorage;
use libra::utils::path;
use mercury::internal::index::Index;

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
        dry_run: false,
        ignore_errors: false,
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

#[tokio::test]
#[serial]
/// Tests basic HEAD detachment capabilities with simple reference paths.
/// Validates relative references (HEAD^, HEAD~), numeric references (HEAD^1, HEAD~1),
/// and complex reference combinations (HEAD^^^, HEAD~~~, HEAD^~^~).
async fn test_detach_head_basic() {
    println!("\n\x1b[1mTest detach use the head's ref basically.\x1b[0m");
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());
    println!("temp_path {:?}", temp_path);

    for i in 0..6 {
        let args = CommitArgs {
            message: format!("commit_{}", i),
            allow_empty: true,
            conventional: false,
            amend: false,
        };
        commit::execute(args).await;
    }
    //detach to head
    {
        switch_to_branch("master".to_string()).await;

        let commit_message = switch_to_detach("HEAD".to_string()).await;
        assert_eq!(&commit_message, "commit_5");
    }

    //detach to the before commit
    {
        let commit_message = switch_to_detach("HEAD^".to_string()).await;
        assert_eq!(&commit_message, "commit_4");
    }

    {
        let commit_message = switch_to_detach("HEAD~".to_string()).await;
        assert_eq!(&commit_message, "commit_3");
    }
    {
        let commit_message = switch_to_detach("HEAD^1".to_string()).await;
        assert_eq!(&commit_message, "commit_2");
    }

    {
        let commit_message = switch_to_detach("HEAD~1".to_string()).await;
        assert_eq!(&commit_message, "commit_1");
    }
    switch_to_branch("master".to_string()).await;

    for i in 6..12 {
        let args = CommitArgs {
            message: format!("commit_{}", i),
            allow_empty: true,
            conventional: false,
            amend: false,
        };
        commit::execute(args).await;
    }

    //detach use head's ref
    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach("HEAD~11".to_string()).await;
        assert_eq!(&commit_message, "commit_0");
    }
    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach("HEAD~~~~~~~~~~~".to_string()).await;
        assert_eq!(&commit_message, "commit_0");
    }
    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach("HEAD^^^^^^^^^^^".to_string()).await;
        assert_eq!(&commit_message, "commit_0");
    }

    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach("HEAD^~^~^~^~^~^".to_string()).await;
        assert_eq!(&commit_message, "commit_0");
    }
    //detach use branch's ref
    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach("master~11".to_string()).await;
        assert_eq!(&commit_message, "commit_0");
    }
    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach("master~~~~~~~~~~~".to_string()).await;
        assert_eq!(&commit_message, "commit_0");
    }
    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach("master^^^^^^^^^^^".to_string()).await;
        assert_eq!(&commit_message, "commit_0");
    }

    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach("master^~^~^~^~^~^".to_string()).await;
        assert_eq!(&commit_message, "commit_0");
    }
    let master_commit_id = Branch::find_branch("master", None).await.unwrap().commit;
    //detach use commit's ref
    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach(format!("{}~11", master_commit_id)).await;
        assert_eq!(&commit_message, "commit_0");
    }
    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach(format!("{}~11", master_commit_id)).await;
        assert_eq!(&commit_message, "commit_0");
    }
    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach(format!("{}~~~~~~~~~~~", master_commit_id)).await;
        assert_eq!(&commit_message, "commit_0");
    }
    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach(format!("{}^^^^^^^^^^^", master_commit_id)).await;
        assert_eq!(&commit_message, "commit_0");
    }

    {
        switch_to_branch("master".to_string()).await;
        let commit_message = switch_to_detach(format!("{}^~^~^~^~^~^", master_commit_id)).await;
        assert_eq!(&commit_message, "commit_0");
    }
}

// a tree with many parents.
async fn create_commit_tree() {
    let index = Index::load(path::index()).unwrap();
    let storage = ClientStorage::init(path::objects());

    let tree = commit::create_tree(&index, &storage, "".into()).await;

    let mut commit_1 = Commit::from_tree_id(tree.id, vec![], &format_commit_msg("commit_0", None));
    commit_1.committer.timestamp = 1;
    save_object(&commit_1, &commit_1.id).unwrap();

    let mut parents_ids = vec![];
    for i in 1..12 {
        let tree = commit::create_tree(&index, &storage, "".into()).await;

        let mut commit = Commit::from_tree_id(
            tree.id,
            vec![commit_1.id],
            &format_commit_msg(&format!("commit_{}", i), None),
        );
        commit.committer.timestamp = (i + 1) as usize;
        save_object(&commit, &commit.id).unwrap();
        parents_ids.push(commit.id);
    }
    {
        let tree = commit::create_tree(&index, &storage, "".into()).await;

        let mut commit_last = Commit::from_tree_id(
            tree.id,
            parents_ids,
            &format_commit_msg("commit_last", None),
        );
        commit_last.committer.timestamp = 100;
        save_object(&commit_last, &commit_last.id).unwrap();
        Branch::update_branch("master", &commit_last.id.to_string(), None).await;
    }
}

#[tokio::test]
#[serial]
// Comprehensive tests for HEAD reference navigation using Git-style paths
// Validates support for ^ (parent selection), ~ (ancestry traversal), and their combinations
async fn test_detach_head_extra() {
    println!("\n\x1b[1mTest detach use the head's ref extra.\x1b[0m");
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());
    println!("temp_path {:?}", temp_path);

    create_commit_tree().await;
    //detach to head
    {
        let commit_message = switch_to_detach("HEAD".to_string()).await;
        assert_eq!(commit_message, "commit_last".to_string());
    }

    for i in 1..12 {
        let commit_message = switch_to_detach(format!("HEAD^{}", i)).await;
        assert_eq!(commit_message, format!("commit_{}", i));

        //back to the last commit
        switch_to_branch("master".to_string()).await;
    }
    //detach use the branch's ref
    for i in 1..12 {
        let commit_message = switch_to_detach(format!("master^{}", i)).await;
        assert_eq!(commit_message, format!("commit_{}", i));

        //back to the last commit
        switch_to_branch("master".to_string()).await;
    }
    //detach use head's ref
    {
        let commit_message = switch_to_detach("HEAD^11~".to_string()).await;
        assert_eq!(commit_message, "commit_0".to_string());
        switch_to_branch("master".to_string()).await;
    }
    //detach use branch's ref
    {
        let commit_message = switch_to_detach("master^11~".to_string()).await;
        assert_eq!(commit_message, "commit_0".to_string());
        switch_to_branch("master".to_string()).await;
    }
    let master_commit_id = Branch::find_branch("master", None).await.unwrap().commit;
    //detach use commit's ref
    {
        let commit_message = switch_to_detach(format!("{}^11~", master_commit_id)).await;
        assert_eq!(commit_message, "commit_0".to_string());
        switch_to_branch("master".to_string()).await;
    }
}

async fn switch_to_detach(branch_test: String) -> String {
    let args = SwitchArgs {
        branch: Some(branch_test),
        create: None,
        detach: true,
    };
    switch::execute(args).await;
    let head = Head::current().await;
    let commit_id = match head {
        Head::Detached(commit) => commit,
        _ => panic!("head not detached,unreachable"),
    };
    let commit = load_object::<Commit>(&commit_id).unwrap();
    commit.message.trim().to_string()
}

async fn switch_to_branch(branch_test: String) {
    let args = SwitchArgs {
        branch: Some(branch_test),
        create: None,
        detach: false,
    };
    switch::execute(args).await;
}
