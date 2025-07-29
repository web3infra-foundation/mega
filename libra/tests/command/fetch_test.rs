use std::process::Command;
use tempfile::TempDir;

/// Helper function: Initialize a temporary Libra repository
fn init_temp_repo() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let temp_path = temp_dir.path();

    println!("Temporary directory created at: {:?}", temp_path);
    assert!(temp_path.is_dir(), "Temporary path is not a valid directory");

    let output = Command::new("libra")
        .current_dir(temp_path)
        .arg("init")
        .output()
        .expect("Failed to execute libra binary");

    if !output.status.success() {
        panic!(
            "Failed to initialize libra repository: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    temp_dir
}

#[tokio::test]
/// Test fetching the default remote repository without parameters
async fn test_fetch_default_remote() {
    let temp_repo = init_temp_repo();
    let temp_path = temp_repo.path();

    // Configure the default remote repository
    Command::new("libra")
        .current_dir(temp_path)
        .args(&["remote", "add", "origin", "https://example.com/repo.git"])
        .output()
        .expect("Failed to add remote");

    // Set the default branch
    Command::new("libra")
        .current_dir(temp_path)
        .args(&["branch", "--set-upstream-to", "origin/main"])
        .output()
        .expect("Failed to set upstream branch");

    // Fetch the default remote repository
    let output = Command::new("libra")
        .current_dir(temp_path)
        .arg("fetch")
        .output()
        .expect("Failed to execute libra fetch");

    assert!(
        output.status.success(),
        "Fetch default remote failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
/// Test fetching from an invalid remote repository
async fn test_fetch_invalid_remote() {
    let temp_repo = init_temp_repo();
    let temp_path = temp_repo.path();

    // Configure an invalid remote repository
    Command::new("libra")
        .current_dir(temp_path)
        .args(&["remote", "add", "origin", "https://invalid-url/repo.git"])
        .output()
        .expect("Failed to add remote");

    // Set the default branch
    Command::new("libra")
        .current_dir(temp_path)
        .args(&["branch", "--set-upstream-to", "origin/main"])
        .output()
        .expect("Failed to set upstream branch");

    // Attempt to fetch from the invalid remote repository
    let output = Command::new("libra")
        .current_dir(temp_path)
        .arg("fetch")
        .output()
        .expect("Failed to execute libra fetch");

    assert!(
        !output.status.success(),
        "Fetch should fail for invalid remote"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.is_empty(),
        "Expected an error message in stderr, but it was empty"
    );
}

#[tokio::test]
/// Test fetching a nonexistent branch
async fn test_fetch_nonexistent_branch() {
    let temp_repo = init_temp_repo();
    let temp_path = temp_repo.path();

    // Configure the remote repository
    Command::new("libra")
        .current_dir(temp_path)
        .args(&["remote", "add", "origin", "https://example.com/repo.git"])
        .output()
        .expect("Failed to add remote");

    // Set the default branch
    Command::new("libra")
        .current_dir(temp_path)
        .args(&["branch", "--set-upstream-to", "origin/main"])
        .output()
        .expect("Failed to set upstream branch");

    // Attempt to fetch a nonexistent branch
    let output = Command::new("libra")
        .current_dir(temp_path)
        .args(&["fetch", "origin", "nonexistent-branch"])
        .output()
        .expect("Failed to execute libra fetch");

    // Check if fetch failed
    assert!(
        !output.status.success(),
        "Fetch should fail for nonexistent branch"
    );

    // Check the error message
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("fatal: couldn't find remote ref nonexistent-branch"),
        "Expected error for nonexistent branch, but got: {}",
        stderr
    );
}
