use std::process::Command;
use tempfile::TempDir;

/// Helper function: Initialize a temporary Libra repository
fn init_temp_repo() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let temp_path = temp_dir.path();

    // 变量可以直接在 `format!` 字符串中使用
    println!("Temporary directory created at: {temp_path:?}");
    assert!(temp_path.is_dir(), "Temporary path is not a valid directory");

    // 修改这一行：使用 env!("CARGO_BIN_EXE_libra") 来获取 libra 可执行文件的路径
    let output = Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .arg("init")
        .output()
        .expect("Failed to execute libra binary"); // 错误信息保持不变，但现在应该能找到文件了

    if !output.status.success() {
        panic!(
            "Failed to initialize libra repository: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    temp_dir
}

#[tokio::test]
/// Test track/untrack path rule management
async fn test_lfs_track_untrack() {
    let temp_repo = init_temp_repo();
    let temp_path = temp_repo.path();

    // Add a path rule
    let track_output = Command::new("libra")
        .current_dir(temp_path)
        .args(&["lfs", "track", "*.txt"])
        .output()
        .expect("Failed to track path");
    assert!(
        track_output.status.success(),
        "Failed to track path: {}",
        String::from_utf8_lossy(&track_output.stderr)
    );

    // Remove a path rule
    let untrack_output = Command::new("libra")
        .current_dir(temp_path)
        .args(&["lfs", "untrack", "*.txt"])
        .output()
        .expect("Failed to untrack path");
    assert!(
        untrack_output.status.success(),
        "Failed to untrack path: {}",
        String::from_utf8_lossy(&untrack_output.stderr)
    );
}

#[tokio::test]
/// Test file status viewing
async fn test_lfs_ls_files() {
    let temp_repo = init_temp_repo();
    let temp_path = temp_repo.path();

    // Create a test file and add it to LFS
    let file_path = temp_path.join("tracked_file.txt");
    std::fs::write(&file_path, "Tracked content").expect("Failed to create tracked file");
    Command::new("libra")
        .current_dir(temp_path)
        .args(&["lfs", "track", "*.txt"])
        .output()
        .expect("Failed to track file");
    Command::new("libra")
        .current_dir(temp_path)
        .args(&["add", "tracked_file.txt"])
        .output()
        .expect("Failed to add file to LFS");

    // View file status
    let ls_files_output = Command::new("libra")
        .current_dir(temp_path)
        .args(&["lfs", "ls-files"])
        .output()
        .expect("Failed to list LFS files");
    assert!(
        ls_files_output.status.success(),
        "Failed to list LFS files: {}",
        String::from_utf8_lossy(&ls_files_output.stderr)
    );

    let stdout = String::from_utf8_lossy(&ls_files_output.stdout);
    assert!(
        stdout.contains("tracked_file.txt"),
        "LFS file list does not contain expected file: {}",
        stdout
    );
}
