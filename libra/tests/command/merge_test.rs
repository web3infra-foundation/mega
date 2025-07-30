use std::process::Command;
use tempfile::TempDir;

/// Helper function: Initialize a temporary Libra repository
fn init_temp_repo() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let temp_path = temp_dir.path();

    // 变量可以直接在 `format!` 字符串中使用
    // FIX: 更新 println! 格式
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
/// Test fast-forward merge of local branches
async fn test_merge_fast_forward() {
    let temp_repo = init_temp_repo();
    let temp_path = temp_repo.path();

    // Create and switch to the feature branch
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["branch", "feature"])
        .output()
        .expect("Failed to create branch");
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["checkout", "feature"])
        .output()
        .expect("Failed to checkout branch");

    // Commit changes on the feature branch
    let file_path = temp_path.join("file.txt");
    std::fs::write(&file_path, "Feature content").expect("Failed to write file");
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["add", "."])
        .output()
        .expect("Failed to add file");
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["commit", "-m", "Add feature content"])
        .output()
        .expect("Failed to commit");

    // Switch back to the main branch and perform fast-forward merge
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["checkout", "main"])
        .output()
        .expect("Failed to checkout main branch");
    // FIX: 移除 &
    let merge_output = Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["merge", "feature"])
        .output()
        .expect("Failed to merge branch");
    assert!(
        merge_output.status.success(),
        "Fast-forward merge failed: {}",
        String::from_utf8_lossy(&merge_output.stderr)
    );
}


#[tokio::test]
/// Test merging a remote branch
async fn test_merge_remote_branch() {
    let temp_repo = init_temp_repo();
    let temp_path = temp_repo.path();

    // Simulate adding a remote branch
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["remote", "add", "origin", "https://example.com/repo.git"])
        .output()
        .expect("Failed to add remote");

    // Merge the remote branch
    // FIX: 移除 &
    let merge_output = Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["merge", "origin/feature"])
        .output()
        .expect("Failed to merge remote branch");
    assert!(
        merge_output.status.success(),
        "Merge remote branch failed: {}",
        String::from_utf8_lossy(&merge_output.stderr)
    );
}


#[tokio::test]
/// Test merging branches with no common ancestor
async fn test_merge_no_common_ancestor() {
    let temp_repo = init_temp_repo();
    let temp_path = temp_repo.path();

    // Create and switch to branch1
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["branch", "branch1"])
        .output()
        .expect("Failed to create branch");
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["checkout", "branch1"])
        .output()
        .expect("Failed to checkout branch");

    // Commit changes on branch1
    let branch1_file = temp_path.join("branch1.txt");
    std::fs::write(&branch1_file, "Branch1 content").expect("Failed to write file");
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["add", "."])
        .output()
        .expect("Failed to add file");
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["commit", "-m", "Add branch1 content"])
        .output()
        .expect("Failed to commit");

    // Create and switch to branch2
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["checkout", "-b", "branch2", "HEAD~1"])
        .output()
        .expect("Failed to create branch");

    // Commit changes on branch2
    let branch2_file = temp_path.join("branch2.txt");
    std::fs::write(&branch2_file, "Branch2 content").expect("Failed to write file");
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["add", "."])
        .output()
        .expect("Failed to add file");
    // FIX: 移除 &
    Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["commit", "-m", "Add branch2 content"])
        .output()
        .expect("Failed to commit");

    // Attempt to merge branches with no common ancestor
    // FIX: 移除 &
    let merge_output = Command::new(env!("CARGO_BIN_EXE_libra"))
        .current_dir(temp_path)
        .args(["merge", "branch2"])
        .output()
        .expect("Failed to merge branch");
    assert!(
        merge_output.status.success(),
        "Merge no common ancestor branch failed: {}",
        String::from_utf8_lossy(&merge_output.stderr)
    );
}

