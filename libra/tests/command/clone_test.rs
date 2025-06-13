use libra::command;
use libra::command::clone::CloneArgs;
use libra::internal::head::Head;
use libra::utils::test;
use serial_test::serial;
use tempfile::tempdir;

#[tokio::test]
#[serial]
#[ignore]
/// Test the clone command with a specific branch
async fn test_clone_branch() {
    let temp_path = tempdir().unwrap();
    let _guard = test::ChangeDirGuard::new(temp_path.path());

    let remote_url = "https://gitee.com/pikady/mega-libra-clone-branch-test.git".to_string();

    command::clone::execute(CloneArgs {
        remote_repo: remote_url,
        local_path: Some(temp_path.path().to_str().unwrap().to_string()),
        branch: Some("dev".to_string()),
    })
    .await;

    // Verify that the `.libra` directory exists
    let libra_dir = temp_path.path().join(".libra");
    assert!(libra_dir.exists());

    // Verify the Head reference
    match Head::current().await {
        Head::Branch(current_branch) => {
            assert_eq!(current_branch, "dev");
        }
        _ => panic!("should be branch"),
    };
}

#[tokio::test]
#[serial]
#[ignore]
/// Test the clone command with the default branch
async fn test_clone_default_branch() {
    let temp_path = tempdir().unwrap();
    let _guard = test::ChangeDirGuard::new(temp_path.path());

    let remote_url = "https://gitee.com/pikady/mega-libra-clone-branch-test.git".to_string();

    command::clone::execute(CloneArgs {
        remote_repo: remote_url,
        local_path: Some(temp_path.path().to_str().unwrap().to_string()),
        branch: None,
    })
    .await;

    // Verify that the `.libra` directory exists
    let libra_dir = temp_path.path().join(".libra");
    assert!(libra_dir.exists());

    // Verify the Head reference
    match Head::current().await {
        Head::Branch(current_branch) => {
            assert_eq!(current_branch, "master");
        }
        _ => panic!("should be branch"),
    };
}

#[tokio::test]
#[serial]
#[ignore]
/// Test the clone command with an empty repository
async fn test_clone_empty_repo() {
    let temp_path = tempdir().unwrap();
    let _guard = test::ChangeDirGuard::new(temp_path.path());

    let remote_url = "https://gitee.com/pikady/mega-libra-empty-repo.git".to_string();

    command::clone::execute(CloneArgs {
        remote_repo: remote_url,
        local_path: Some(temp_path.path().to_str().unwrap().to_string()),
        branch: None,
    })
    .await;

    // Verify that the `.libra` directory exists
    let libra_dir = temp_path.path().join(".libra");
    assert!(libra_dir.exists());

    // Verify the Head reference
    match Head::current().await {
        Head::Branch(current_branch) => {
            assert_eq!(current_branch, "master");
        }
        _ => panic!("should be branch"),
    };
}
