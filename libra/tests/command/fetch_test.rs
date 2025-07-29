use std::process::Command;
use tempfile::TempDir;

/// Helper function: Initialize a temporary Libra repository
fn init_temp_repo() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let temp_path = temp_dir.path();

    // 变量可以直接在 `format!` 字符串中使用
    println!("Temporary directory created at: {temp_path:?}");
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
/// Test fetching from an invalid remote repository
async fn test_fetch_invalid_remote() {
    let temp_repo = init_temp_repo();
    let temp_path = temp_repo.path();

    // Configure an invalid remote repository
    // 借用的表达式实现了所需的 trait
    Command::new("libra")
        .current_dir(temp_path)
        .args(["remote", "add", "origin", "https://invalid-url/repo.git"]) // 移除了 &
        .output()
        .expect("Failed to add remote");

    // Set the default branch
    // 借用的表达式实现了所需的 trait
    Command::new("libra")
        .current_dir(temp_path)
        .args(["branch", "--set-upstream-to", "origin/main"]) // 移除了 &
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
    // 借用的表达式实现了所需的 trait
    Command::new("libra")
        .current_dir(temp_path)
        .args(["remote", "add", "origin", "https://example.com/repo.git"]) // 移除了 &
        .output()
        .expect("Failed to add remote");

    // Set the default branch
    // 借用的表达式实现了所需的 trait
    Command::new("libra")
        .current_dir(temp_path)
        .args(["branch", "--set-upstream-to", "origin/main"]) // 移除了 &
        .output()
        .expect("Failed to set upstream branch");

    // Attempt to fetch a nonexistent branch
    // 借用的表达式实现了所需的 trait
    let output = Command::new("libra")
        .current_dir(temp_path)
        .args(["fetch", "origin", "nonexistent-branch"]) // 移除了 &
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
        !stderr.is_empty(),
        "Expected an error message for nonexistent branch, but stderr was empty"
    );
}
