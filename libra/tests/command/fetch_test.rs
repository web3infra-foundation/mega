use std::process::Command;
use tempfile::TempDir;

/// Helper function: Initialize a temporary Libra repository
fn init_temp_repo() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let temp_path = temp_dir.path();

    println!("Temporary directory created at: {temp_path:?}");
    assert!(temp_path.is_dir(), "Temporary path is not a valid directory");

    let output = Command::new(env!("CARGO_BIN_EXE_libra"))
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
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["remote", "add", "origin", "https://invalid-url/repo.git"]) // 移除了 &
        .output()
        .expect("Failed to add remote");

    // Set the default branch
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["branch", "--set-upstream-to", "origin/main"])
        .output()
        .expect("Failed to set upstream branch");

    // Attempt to fetch from the invalid remote repository
    let output = Command::new(env!("CARGO_BIN_EXE_libra"))
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
