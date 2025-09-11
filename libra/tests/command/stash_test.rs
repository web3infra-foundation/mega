#![cfg(test)]
use super::*;
use libra::cli::Stash;
use libra::command::stash;
use serial_test::serial;
use std::fs;
use std::io::Write;
use tempfile::tempdir;

#[tokio::test]
#[serial]
/// Test basic functionality of stash push
async fn test_stash_push() {
    // 1. Prepare the test environment
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a file and modify it
    let file_path = "test_file.txt";
    let mut file = fs::File::create(file_path).unwrap();
    file.write_all(b"Original content").unwrap();
    
    // Add to staging area and commit
    add::execute(AddArgs {
        pathspec: vec![file_path.to_string()],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    }).await;
    
    commit::execute(CommitArgs {
        message: "Initial commit".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: true,
    }).await;

    // Modify the file
    let mut file = fs::OpenOptions::new().write(true).truncate(true).open(file_path).unwrap();
    file.write_all(b"Modified content").unwrap();

    // 2. Execute stash push
    stash::execute(Stash::Push { 
        message: Some("Test stash".to_string()) 
    }).await.unwrap();

    // 3. Verify the results
    // Check if the working directory is clean
    let changes = changes_to_be_staged();
    assert!(changes.new.is_empty());
    assert!(changes.modified.is_empty());
    
    // Check if the file content is restored to the committed state
    let content = fs::read_to_string(file_path).unwrap();
    assert_eq!(content, "Original content");
}

#[tokio::test]
#[serial]
/// Test stash list functionality
async fn test_stash_list() {
    // 1. Prepare the environment
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create and modify a file
    let file_path = "test_file.txt";
    let mut file = fs::File::create(file_path).unwrap();
    file.write_all(b"Content").unwrap();
    
    add::execute(AddArgs {
        pathspec: vec![file_path.to_string()],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    }).await;
    
    commit::execute(CommitArgs {
        message: "Initial commit".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: true,
    }).await;

    // Modify the file and create a stash
    let mut file = fs::OpenOptions::new().write(true).truncate(true).open(file_path).unwrap();
    file.write_all(b"Modified").unwrap();
    
    stash::execute(Stash::Push { 
        message: Some("Test stash 1".to_string()) 
    }).await.unwrap();

    // Modify again and create another stash
    let mut file = fs::OpenOptions::new().write(true).truncate(true).open(file_path).unwrap();
    file.write_all(b"Modified again").unwrap();
    
    stash::execute(Stash::Push { 
        message: Some("Test stash 2".to_string()) 
    }).await.unwrap();

    // 2. Execute stash list (here we need to capture output or verify internal state)
    // Since the list command primarily prints output, we can verify it does not error
    stash::execute(Stash::List).await.unwrap();
}

#[tokio::test]
#[serial]
/// Test stash branch functionality (key test!)
async fn test_stash_branch() {
    // 1. Prepare the environment
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create initial file and commit
    let file_path = "test_file.txt";
    let mut file = fs::File::create(file_path).unwrap();
    file.write_all(b"Original content").unwrap();
    
    add::execute(AddArgs {
        pathspec: vec![file_path.to_string()],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    }).await;
    
    commit::execute(CommitArgs {
        message: "Initial commit".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: true,
    }).await;

    // Modify the file and create a stash
    let mut file = fs::OpenOptions::new().write(true).truncate(true).open(file_path).unwrap();
    file.write_all(b"Stashed content").unwrap();
    
    stash::execute(Stash::Push { 
        message: Some("Test stash for branch".to_string()) 
    }).await.unwrap();

    // 2. Execute stash branch
    let branch_name = "stash-branch-test";
    stash::execute(Stash::Branch { 
        stash: None,  // Use the latest stash
        branch: branch_name.to_string() 
    }).await.unwrap();

    // 3. Verify the results
    // Check if the branch was created
    let branch = Branch::find_branch(branch_name, None).await;
    assert!(branch.is_some(), "Branch should be created");

    // Check if the current branch switches to the new branch
    match Head::current().await {
        Head::Branch(current_branch) => {
            assert_eq!(current_branch, branch_name);
        }
        _ => panic!("Should be on a branch"),
    }

    // Check if the file content is the stashed content
    let content = fs::read_to_string(file_path).unwrap();
    assert_eq!(content, "Stashed content");

    // Check if the stash is deleted (stash branch automatically deletes stash)
    // Here we can verify stash existence by attempting to stash pop again
    let result = stash::execute(Stash::Pop { stash: None }).await;
    assert!(result.is_err(), "Stash should be deleted after branch creation");
}

#[tokio::test]
#[serial]
/// Test error case: executing stash branch without any stash
async fn test_stash_branch_no_stash() {
    // 1. Prepare the environment
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // 2. Attempt to execute stash branch without any stash
    let result = stash::execute(Stash::Branch { 
        stash: None,
        branch: "test-branch".to_string() 
    }).await;

    // 3. Verify that an error should be returned
    assert!(result.is_err(), "Should fail when no stash exists");
    
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("No stash entries found"), 
            "Error message should indicate no stash found");
}

#[tokio::test]
#[serial]
/// Test error case: branch name already exists
async fn test_stash_branch_existing_branch() {
    // 1. Prepare the environment
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a branch
    let branch_name = "existing-branch";
    branch::execute(BranchArgs {
        new_branch: Some(branch_name.to_string()),
        commit_hash: None,
        list: false,
        delete: None,
        set_upstream_to: None,
        show_current: false,
        remotes: false,
    }).await;

    // Create a stash
    let file_path = "test_file.txt";
    let mut file = fs::File::create(file_path).unwrap();
    file.write_all(b"Content").unwrap();
    
    add::execute(AddArgs {
        pathspec: vec![file_path.to_string()],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    }).await;
    
    commit::execute(CommitArgs {
        message: "Initial commit".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: true,
    }).await;

    let mut file = fs::OpenOptions::new().write(true).truncate(true).open(file_path).unwrap();
    file.write_all(b"Modified").unwrap();
    
    stash::execute(Stash::Push { 
        message: Some("Test stash".to_string()) 
    }).await.unwrap();

    // 2. Attempt to create an already existing branch
    let result = stash::execute(Stash::Branch { 
        stash: None,
        branch: branch_name.to_string() 
    }).await;

    // 3. Verify that an error should be returned
    assert!(result.is_err(), "Should fail when branch already exists");
    
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("already exists"), 
            "Error message should indicate branch already exists");
        }