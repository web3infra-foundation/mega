use std::process::Command;
use tempfile::TempDir;

/// Helper function: Initialize a temporary Libra repository
fn init_temp_repo() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let temp_path = temp_dir.path();

    // Variables can be used directly in the `format!` string
    // FIX: Removed {:?} and added variable directly with formatting
    println!("Temporary directory created at: {temp_path:?}");
    assert!(temp_path.is_dir(), "Temporary path is not a valid directory");

    // Using env!("CARGO_BIN_EXE_libra") to get the path to the libra executable
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
/// Test track/untrack path rule management
async fn test_lfs_track_untrack() {
    let temp_repo = init_temp_repo();
    let temp_path = temp_repo.path();

    // Add a path rule
    // FIX: Removed & from args
    let track_output = Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["lfs", "track", "*.txt"]) // Changed &[...] to [...]
        .output()
        .expect("Failed to track path");
    assert!(
        track_output.status.success(),
        "Failed to track path: {}",
        String::from_utf8_lossy(&track_output.stderr)
    );

    // Remove a path rule
    // FIX: Removed & from args
    let untrack_output = Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["lfs", "untrack", "*.txt"]) // Changed &[...] to [...]
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

    // FIX: Removed & from args
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["lfs", "track", "*.txt"]) // Changed &[...] to [...]
        .output()
        .expect("Failed to track file");

    // FIX: Removed & from args
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["add", "tracked_file.txt"]) // Changed &[...] to [...]
        .output()
        .expect("Failed to add file to LFS");

    // View file status
    // FIX: Removed & from args
    let ls_files_output = Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["lfs", "ls-files"]) // Changed &[...] to [...]
        .output()
        .expect("Failed to list LFS files");
    assert!(
        ls_files_output.status.success(),
        "Failed to list LFS files: {}",
        String::from_utf8_lossy(&ls_files_output.stderr)
    );

    let stdout = String::from_utf8_lossy(&ls_files_output.stdout);
    // FIX: Variables can be used directly in the `format!` string
    assert!(
        stdout.contains("tracked_file.txt"),
        "LFS file list does not contain expected file: {stdout}", // Changed {} to direct variable embed
    );
}
