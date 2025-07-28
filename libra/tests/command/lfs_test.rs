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
/// Test file locking and unlocking
async fn test_lfs_lock_unlock() {
    let temp_repo = init_temp_repo();
    let temp_path = temp_repo.path();

    // Create a test file
    let file_path = temp_path.join("test_file.txt");
    std::fs::write(&file_path, "Test content").expect("Failed to create test file");

    // Lock the file
    let lock_output = Command::new("libra")
        .current_dir(temp_path)
        .args(&["lfs", "lock", "test_file.txt"])
        .output()
        .expect("Failed to lock file");
    assert!(
        lock_output.status.success(),
        "Failed to lock file: {}",
        String::from_utf8_lossy(&lock_output.stderr)
    );

    // Unlock the file
    let unlock_output = Command::new("libra")
        .current_dir(temp_path)
        .args(&["lfs", "unlock", "test_file.txt"])
        .output()
        .expect("Failed to unlock file");
    assert!(
        unlock_output.status.success(),
        "Failed to unlock file: {}",
        String::from_utf8_lossy(&unlock_output.stderr)
    );
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
/// Test lock list query
async fn test_lfs_locks_list() {
    let temp_repo = init_temp_repo();
    let temp_path = temp_repo.path();

    // Create and lock a file
    let file_path = temp_path.join("locked_file.txt");
    std::fs::write(&file_path, "Locked content").expect("Failed to create locked file");
    Command::new("libra")
        .current_dir(temp_path)
        .args(&["lfs", "lock", "locked_file.txt"])
        .output()
        .expect("Failed to lock file");

    // Query the lock list
    let locks_output = Command::new("libra")
        .current_dir(temp_path)
        .args(&["lfs", "locks"])
        .output()
        .expect("Failed to list locks");
    assert!(
        locks_output.status.success(),
        "Failed to list locks: {}",
        String::from_utf8_lossy(&locks_output.stderr)
    );

    let stdout = String::from_utf8_lossy(&locks_output.stdout);
    assert!(
        stdout.contains("locked_file.txt"),
        "Lock list does not contain expected file: {}",
        stdout
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
