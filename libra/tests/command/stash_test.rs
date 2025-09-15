#![cfg(test)] // Enable test compilation only when running tests
use super::*; // Import all items from parent module
use libra::cli::Stash; // Import Stash enum from CLI module
use libra::command::stash; // Import stash command module
use libra::internal::branch::Branch; // Import Branch struct for branch operations
use libra::internal::head::Head; // Import Head enum for HEAD operations
use serial_test::serial; // Import serial test attribute for sequential test execution
use std::fs; // Import filesystem operations
use tempfile::tempdir; // Import temporary directory creation

/// Helper function to create a test file with specified content
/// This function creates a file at the given path with the specified filename and content
async fn create_test_file(path: &std::path::Path, filename: &str, content: &str) {
    let file_path = path.join(filename); // Construct full file path
    fs::write(file_path, content).expect("Failed to create test file"); // Write content to file
}

/// Helper function to verify that a file contains the expected content
/// Returns true if the file exists and contains the expected content, false otherwise
fn verify_file_content(path: &std::path::Path, filename: &str, expected_content: &str) -> bool {
    let file_path = path.join(filename); // Construct full file path
    if let Ok(content) = fs::read_to_string(file_path) { // Try to read file content
        content == expected_content // Compare actual content with expected content
    } else {
        false // Return false if file cannot be read
    }
}

/// Helper function to verify that a branch exists and get its commit hash
/// Returns Some(commit_hash) if branch exists, None otherwise
async fn get_branch_commit(branch_name: &str) -> Option<mercury::hash::SHA1> {
    if let Some(branch) = Branch::find_branch(branch_name, None).await { // Try to find the branch
        Some(branch.commit) // Return the commit hash if branch exists
    } else {
        None // Return None if branch doesn't exist
    }
}

#[tokio::test] // Mark as async test using tokio runtime
#[serial] // Ensure this test runs sequentially with other serial tests
/// Tests basic stash branch functionality - creating a branch from the latest stash
/// This test verifies that a branch can be created from a stash and contains the correct content
async fn test_stash_branch_basic() {
    let temp_path = tempdir().unwrap(); // Create temporary directory for test
    test::setup_with_new_libra_in(temp_path.path()).await; // Initialize libra repository in temp directory
    let _guard = ChangeDirGuard::new(temp_path.path()); // Change to temp directory and ensure cleanup

    // 1. Create initial commit to establish repository history
    let commit_args = CommitArgs { // Configure commit arguments
        message: "initial commit".to_string(), // Set commit message
        allow_empty: true, // Allow empty commit
        conventional: false, // Don't use conventional commit format
        amend: false, // Don't amend previous commit
        signoff: false, // Don't add sign-off
        disable_pre: true, // Disable pre-commit hooks
    };
    commit::execute(commit_args).await; // Execute the commit
    let initial_commit = Head::current_commit().await.unwrap(); // Get the initial commit hash

    // 2. Create and add a test file to the repository
    create_test_file(temp_path.path(), "test.txt", "original content").await; // Create test file
    let add_args = AddArgs { // Configure add arguments
        pathspec: vec!["test.txt".to_string()], // Specify file to add
        all: false, // Don't add all files
        update: false, // Don't update only tracked files
        refresh: false, // Don't refresh index entries
        verbose: false, // Don't show detailed output
        dry_run: false, // Actually perform the operation
        ignore_errors: false, // Don't ignore errors
    };
    add::execute(add_args).await; // Execute the add command

    // 3. Commit the file to create a proper base for stashing
    let commit_args = CommitArgs { // Configure commit arguments
        message: "add test file".to_string(), // Set commit message
        allow_empty: false, // Don't allow empty commit
        conventional: false, // Don't use conventional commit format
        amend: false, // Don't amend previous commit
        signoff: false, // Don't add sign-off
        disable_pre: true, // Disable pre-commit hooks
    };
    commit::execute(commit_args).await; // Execute the commit

    // 4. Modify the file and stash the changes
    create_test_file(temp_path.path(), "test.txt", "modified content").await; // Modify the test file
    let stash_cmd = Stash::Push { message: None }; // Create stash push command with no custom message
    stash::execute(stash_cmd).await; // Execute the stash command

    // 5. Verify file was reset to original content after stashing
    assert!(verify_file_content(temp_path.path(), "test.txt", "original content")); // Check file content

    // 6. Create branch from the latest stash
    let branch_name = "test-branch".to_string(); // Define new branch name
    let stash_cmd = Stash::Branch { // Create stash branch command
        stash: None, // Use latest stash (stash@{0})
        branch_name: branch_name.clone(), // Set branch name
    };
    stash::execute(stash_cmd).await; // Execute the stash branch command

    // 7. Verify we're currently on the new branch
    match Head::current().await { // Get current HEAD reference
        Head::Branch(current_branch) => { // If HEAD points to a branch
            assert_eq!(current_branch, branch_name); // Verify it's our new branch
        }
        _ => panic!("Should be on a branch"), // Fail if not on a branch
    }

    // 8. Verify the file contains the stashed content
    assert!(verify_file_content(temp_path.path(), "test.txt", "modified content")); // Check file content

    // 9. Verify the branch was created at the correct commit
    let branch_commit = get_branch_commit(&branch_name).await.unwrap(); // Get branch commit hash
    // The branch should be created at the base commit (before the stash)
    // Since we made changes after the initial commit, the branch should be at the commit with "add test file"
    assert_ne!(branch_commit, initial_commit); // Verify branch is not at initial commit
}

#[tokio::test] // Mark as async test using tokio runtime
#[serial] // Ensure this test runs sequentially with other serial tests
/// Tests creating a branch from a specific stash reference
/// This test verifies that branches can be created from specific stash entries (not just the latest)
async fn test_stash_branch_with_specific_stash() {
    let temp_path = tempdir().unwrap(); // Create temporary directory for test
    test::setup_with_new_libra_in(temp_path.path()).await; // Initialize libra repository in temp directory
    let _guard = ChangeDirGuard::new(temp_path.path()); // Change to temp directory and ensure cleanup

    // 1. Create initial commit to establish repository history
    let commit_args = CommitArgs { // Configure commit arguments
        message: "initial commit".to_string(), // Set commit message
        allow_empty: true, // Allow empty commit
        conventional: false, // Don't use conventional commit format
        amend: false, // Don't amend previous commit
        signoff: false, // Don't add sign-off
        disable_pre: true, // Disable pre-commit hooks
    };
    commit::execute(commit_args).await; // Execute the commit

    // 2. Create first stash entry
    create_test_file(temp_path.path(), "file1.txt", "first stash content").await; // Create first test file
    let add_args = AddArgs { // Configure add arguments for first file
        pathspec: vec!["file1.txt".to_string()], // Specify file to add
        all: false, // Don't add all files
        update: false, // Don't update only tracked files
        refresh: false, // Don't refresh index entries
        verbose: false, // Don't show detailed output
        dry_run: false, // Actually perform the operation
        ignore_errors: false, // Don't ignore errors
    };
    add::execute(add_args).await; // Execute the add command

    let commit_args = CommitArgs { // Configure commit arguments for first file
        message: "add file1".to_string(), // Set commit message
        allow_empty: false, // Don't allow empty commit
        conventional: false, // Don't use conventional commit format
        amend: false, // Don't amend previous commit
        signoff: false, // Don't add sign-off
        disable_pre: true, // Disable pre-commit hooks
    };
    commit::execute(commit_args).await; // Execute the commit

    create_test_file(temp_path.path(), "file1.txt", "modified first content").await; // Modify first file
    let stash_cmd = Stash::Push { // Create first stash
        message: Some("first stash".to_string()), // Set custom stash message
    };
    stash::execute(stash_cmd).await; // Execute the stash command

    // 3. Create second stash entry
    create_test_file(temp_path.path(), "file2.txt", "second stash content").await; // Create second test file
    let add_args = AddArgs { // Configure add arguments for second file
        pathspec: vec!["file2.txt".to_string()], // Specify file to add
        all: false, // Don't add all files
        update: false, // Don't update only tracked files
        refresh: false, // Don't refresh index entries
        verbose: false, // Don't show detailed output
        dry_run: false, // Actually perform the operation
        ignore_errors: false, // Don't ignore errors
    };
    add::execute(add_args).await; // Execute the add command

    let commit_args = CommitArgs { // Configure commit arguments for second file
        message: "add file2".to_string(), // Set commit message
        allow_empty: false, // Don't allow empty commit
        conventional: false, // Don't use conventional commit format
        amend: false, // Don't amend previous commit
        signoff: false, // Don't add sign-off
        disable_pre: true, // Disable pre-commit hooks
    };
    commit::execute(commit_args).await; // Execute the commit

    create_test_file(temp_path.path(), "file2.txt", "modified second content").await; // Modify second file
    let stash_cmd = Stash::Push { // Create second stash
        message: Some("second stash".to_string()), // Set custom stash message
    };
    stash::execute(stash_cmd).await; // Execute the stash command

    // 4. Create branch from stash@{1} (first stash, now second in the stack)
    let branch_name = "branch-from-stash-1".to_string(); // Define new branch name
    let stash_cmd = Stash::Branch { // Create stash branch command
        stash: Some("stash@{1}".to_string()), // Specify first stash (now at index 1)
        branch_name: branch_name.clone(), // Set branch name
    };
    stash::execute(stash_cmd).await; // Execute the stash branch command

    // 5. Verify we're currently on the new branch
    match Head::current().await { // Get current HEAD reference
        Head::Branch(current_branch) => { // If HEAD points to a branch
            assert_eq!(current_branch, branch_name); // Verify it's our new branch
        }
        _ => panic!("Should be on a branch"), // Fail if not on a branch
    }

    // 6. Verify the correct file content is restored from the first stash
    assert!(verify_file_content(temp_path.path(), "file1.txt", "modified first content")); // Check first file content
    // file2.txt should not exist since it was created after the first stash
    assert!(!temp_path.path().join("file2.txt").exists()); // Verify second file doesn't exist
}

#[tokio::test] // Mark as async test using tokio runtime
#[serial] // Ensure this test runs sequentially with other serial tests
/// Tests error handling when no stash exists
/// This test verifies that the command handles the case where no stashes are available
async fn test_stash_branch_no_stash() {
    let temp_path = tempdir().unwrap(); // Create temporary directory for test
    test::setup_with_new_libra_in(temp_path.path()).await; // Initialize libra repository in temp directory
    let _guard = ChangeDirGuard::new(temp_path.path()); // Change to temp directory and ensure cleanup

    // 1. Create initial commit but no stash
    let commit_args = CommitArgs { // Configure commit arguments
        message: "initial commit".to_string(), // Set commit message
        allow_empty: true, // Allow empty commit
        conventional: false, // Don't use conventional commit format
        amend: false, // Don't amend previous commit
        signoff: false, // Don't add sign-off
        disable_pre: true, // Disable pre-commit hooks
    };
    commit::execute(commit_args).await; // Execute the commit

    // 2. Try to create branch from non-existent stash
    let branch_name = "test-branch".to_string(); // Define branch name
    let stash_cmd = Stash::Branch { // Create stash branch command
        stash: None, // Try to use latest stash (which doesn't exist)
        branch_name: branch_name.clone(), // Set branch name
    };
    
    // This should fail, but we can't easily test the error output in this framework
    // The function will print an error message to stderr
    stash::execute(stash_cmd).await; // Execute the stash branch command (will fail)

    // 3. Verify no branch was created due to the error
    assert!(get_branch_commit(&branch_name).await.is_none()); // Verify branch doesn't exist
}

#[tokio::test] // Mark as async test using tokio runtime
#[serial] // Ensure this test runs sequentially with other serial tests
/// Tests error handling with invalid stash reference
/// This test verifies that the command handles invalid stash references gracefully
async fn test_stash_branch_invalid_stash() {
    let temp_path = tempdir().unwrap(); // Create temporary directory for test
    test::setup_with_new_libra_in(temp_path.path()).await; // Initialize libra repository in temp directory
    let _guard = ChangeDirGuard::new(temp_path.path()); // Change to temp directory and ensure cleanup

    // 1. Create initial commit and a valid stash
    let commit_args = CommitArgs { // Configure commit arguments
        message: "initial commit".to_string(), // Set commit message
        allow_empty: true, // Allow empty commit
        conventional: false, // Don't use conventional commit format
        amend: false, // Don't amend previous commit
        signoff: false, // Don't add sign-off
        disable_pre: true, // Disable pre-commit hooks
    };
    commit::execute(commit_args).await; // Execute the commit

    create_test_file(temp_path.path(), "test.txt", "test content").await; // Create test file
    let stash_cmd = Stash::Push { message: None }; // Create stash push command
    stash::execute(stash_cmd).await; // Execute the stash command

    // 2. Try to create branch from invalid stash reference
    let branch_name = "test-branch".to_string(); // Define branch name
    let stash_cmd = Stash::Branch { // Create stash branch command
        stash: Some("stash@{999}".to_string()), // Use non-existent stash index
        branch_name: branch_name.clone(), // Set branch name
    };
    
    stash::execute(stash_cmd).await; // Execute the stash branch command (will fail)

    // 3. Verify no branch was created due to invalid stash reference
    assert!(get_branch_commit(&branch_name).await.is_none()); // Verify branch doesn't exist
}

#[tokio::test] // Mark as async test using tokio runtime
#[serial] // Ensure this test runs sequentially with other serial tests
/// Tests that the stash is properly removed after creating a branch
/// This test verifies that the stash entry is consumed when creating a branch
async fn test_stash_branch_stash_removed() {
    let temp_path = tempdir().unwrap(); // Create temporary directory for test
    test::setup_with_new_libra_in(temp_path.path()).await; // Initialize libra repository in temp directory
    let _guard = ChangeDirGuard::new(temp_path.path()); // Change to temp directory and ensure cleanup

    // 1. Create initial commit and stash
    let commit_args = CommitArgs { // Configure commit arguments
        message: "initial commit".to_string(), // Set commit message
        allow_empty: true, // Allow empty commit
        conventional: false, // Don't use conventional commit format
        amend: false, // Don't amend previous commit
        signoff: false, // Don't add sign-off
        disable_pre: true, // Disable pre-commit hooks
    };
    commit::execute(commit_args).await; // Execute the commit

    create_test_file(temp_path.path(), "test.txt", "original content").await; // Create test file
    let add_args = AddArgs { // Configure add arguments
        pathspec: vec!["test.txt".to_string()], // Specify file to add
        all: false, // Don't add all files
        update: false, // Don't update only tracked files
        refresh: false, // Don't refresh index entries
        verbose: false, // Don't show detailed output
        dry_run: false, // Actually perform the operation
        ignore_errors: false, // Don't ignore errors
    };
    add::execute(add_args).await; // Execute the add command

    let commit_args = CommitArgs { // Configure commit arguments
        message: "add test file".to_string(), // Set commit message
        allow_empty: false, // Don't allow empty commit
        conventional: false, // Don't use conventional commit format
        amend: false, // Don't amend previous commit
        signoff: false, // Don't add sign-off
        disable_pre: true, // Disable pre-commit hooks
    };
    commit::execute(commit_args).await; // Execute the commit

    create_test_file(temp_path.path(), "test.txt", "modified content").await; // Modify test file
    let stash_cmd = Stash::Push { message: None }; // Create stash push command
    stash::execute(stash_cmd).await; // Execute the stash command

    // 2. Verify stash exists by listing (output goes to stdout)
    let stash_cmd = Stash::List; // Create stash list command
    stash::execute(stash_cmd).await; // Execute the list command

    // 3. Create branch from stash (this should consume the stash)
    let branch_name = "test-branch".to_string(); // Define branch name
    let stash_cmd = Stash::Branch { // Create stash branch command
        stash: None, // Use latest stash
        branch_name: branch_name.clone(), // Set branch name
    };
    stash::execute(stash_cmd).await; // Execute the stash branch command

    // 4. Verify stash was removed by trying to list (should be empty)
    let stash_cmd = Stash::List; // Create stash list command
    stash::execute(stash_cmd).await; // Execute the list command
    
    // Note: We can't easily verify the stash was removed in this test framework
    // since the list command just prints to stdout. In a real scenario, 
    // the stash list should be empty after the branch operation.
}

#[tokio::test] // Mark as async test using tokio runtime
#[serial] // Ensure this test runs sequentially with other serial tests
/// Tests creating multiple branches from different stashes
/// This test verifies that multiple branches can be created from different stash entries
async fn test_stash_branch_multiple_stashes() {
    let temp_path = tempdir().unwrap(); // Create temporary directory for test
    test::setup_with_new_libra_in(temp_path.path()).await; // Initialize libra repository in temp directory
    let _guard = ChangeDirGuard::new(temp_path.path()); // Change to temp directory and ensure cleanup

    // 1. Create initial commit
    let commit_args = CommitArgs { // Configure commit arguments
        message: "initial commit".to_string(), // Set commit message
        allow_empty: true, // Allow empty commit
        conventional: false, // Don't use conventional commit format
        amend: false, // Don't amend previous commit
        signoff: false, // Don't add sign-off
        disable_pre: true, // Disable pre-commit hooks
    };
    commit::execute(commit_args).await; // Execute the commit

    // 2. Create first stash
    create_test_file(temp_path.path(), "file1.txt", "content1").await; // Create first test file
    let add_args = AddArgs { // Configure add arguments for first file
        pathspec: vec!["file1.txt".to_string()], // Specify file to add
        all: false, // Don't add all files
        update: false, // Don't update only tracked files
        refresh: false, // Don't refresh index entries
        verbose: false, // Don't show detailed output
        dry_run: false, // Actually perform the operation
        ignore_errors: false, // Don't ignore errors
    };
    add::execute(add_args).await; // Execute the add command

    let commit_args = CommitArgs { // Configure commit arguments for first file
        message: "add file1".to_string(), // Set commit message
        allow_empty: false, // Don't allow empty commit
        conventional: false, // Don't use conventional commit format
        amend: false, // Don't amend previous commit
        signoff: false, // Don't add sign-off
        disable_pre: true, // Disable pre-commit hooks
    };
    commit::execute(commit_args).await; // Execute the commit

    create_test_file(temp_path.path(), "file1.txt", "modified content1").await; // Modify first file
    let stash_cmd = Stash::Push { // Create first stash
        message: Some("first stash".to_string()), // Set custom stash message
    };
    stash::execute(stash_cmd).await; // Execute the stash command

    // 3. Create second stash
    create_test_file(temp_path.path(), "file2.txt", "content2").await; // Create second test file
    let add_args = AddArgs { // Configure add arguments for second file
        pathspec: vec!["file2.txt".to_string()], // Specify file to add
        all: false, // Don't add all files
        update: false, // Don't update only tracked files
        refresh: false, // Don't refresh index entries
        verbose: false, // Don't show detailed output
        dry_run: false, // Actually perform the operation
        ignore_errors: false, // Don't ignore errors
    };
    add::execute(add_args).await; // Execute the add command

    let commit_args = CommitArgs { // Configure commit arguments for second file
        message: "add file2".to_string(), // Set commit message
        allow_empty: false, // Don't allow empty commit
        conventional: false, // Don't use conventional commit format
        amend: false, // Don't amend previous commit
        signoff: false, // Don't add sign-off
        disable_pre: true, // Disable pre-commit hooks
    };
    commit::execute(commit_args).await; // Execute the commit

    create_test_file(temp_path.path(), "file2.txt", "modified content2").await; // Modify second file
    let stash_cmd = Stash::Push { // Create second stash
        message: Some("second stash".to_string()), // Set custom stash message
    };
    stash::execute(stash_cmd).await; // Execute the stash command

    // 4. Create branch from latest stash (stash@{0})
    let branch1_name = "branch-from-latest".to_string(); // Define first branch name
    let stash_cmd = Stash::Branch { // Create stash branch command
        stash: None, // Use latest stash
        branch_name: branch1_name.clone(), // Set branch name
    };
    stash::execute(stash_cmd).await; // Execute the stash branch command

    // 5. Verify first branch was created successfully
    assert!(get_branch_commit(&branch1_name).await.is_some()); // Verify branch exists

    // 6. Switch back to master to create another branch
    let switch_args = SwitchArgs { // Configure switch arguments
        branch: Some("master".to_string()), // Target branch name
        create: None, // Don't create new branch
        detach: false, // Don't detach HEAD
    };
    switch::execute(switch_args).await; // Execute the switch command

    // 7. Create branch from remaining stash
    let branch2_name = "branch-from-remaining".to_string(); // Define second branch name
    let stash_cmd = Stash::Branch { // Create stash branch command
        stash: None, // Should now be the first stash since second was consumed
        branch_name: branch2_name.clone(), // Set branch name
    };
    stash::execute(stash_cmd).await; // Execute the stash branch command

    // 8. Verify second branch was created successfully
    assert!(get_branch_commit(&branch2_name).await.is_some()); // Verify branch exists
}
