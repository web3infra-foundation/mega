use super::*;
// use std::fs::File;
use std::fs;

pub fn verify_init(base_dir: &Path) {
    // List of subdirectories to verify
    let dirs = ["objects/pack", "objects/info", "info"];

    // Loop through the directories and verify they exist
    for dir in dirs {
        let dir_path = base_dir.join(dir);
        assert!(dir_path.exists(), "Directory {} does not exist", dir);
    }

    // Additional file verification
    let files = ["description", "libra.db", "info/exclude"];

    for file in files {
        let file_path = base_dir.join(file);
        assert!(file_path.exists(), "File {} does not exist", file);
    }
}
#[tokio::test]
#[serial]
/// Test the init function with no parameters
async fn test_init() {
    let target_dir = tempdir().unwrap().into_path();
    // let _guard = ChangeDirGuard::new(target_dir.clone());

    let args = InitArgs {
        bare: false,
        initial_branch: None,
        repo_directory: target_dir.to_str().unwrap().to_string(),
        quiet: false,
    };
    // Run the init function
    init(args).await.unwrap();

    // Verify that the `.libra` directory exists
    let libra_dir = target_dir.join(".libra");
    assert!(libra_dir.exists(), ".libra directory does not exist");

    // Verify the contents of the other directory
    verify_init(libra_dir.as_path());
}

#[tokio::test]
#[serial]
/// Test the init function with the --bare flag
async fn test_init_bare() {
    let target_dir = tempdir().unwrap().into_path();
    // let _guard = ChangeDirGuard::new(target_dir.clone());

    // Run the init function with --bare flag
    let args = InitArgs {
        bare: true,
        initial_branch: None,
        repo_directory: target_dir.to_str().unwrap().to_string(),
        quiet: false,
    };
    // Run the init function
    init(args).await.unwrap();

    // Verify the contents of the other directory
    verify_init(target_dir.as_path());
}
#[tokio::test]
#[serial]
/// Test the init function with the --bare flag and an existing repository
async fn test_init_bare_with_existing_repo() {
    let target_dir = tempdir().unwrap().into_path();

    // Initialize a bare repository
    let init_args = InitArgs {
        bare: false,
        initial_branch: None,
        repo_directory: target_dir.to_str().unwrap().to_string(),
        quiet: false,
    };
    init(init_args).await.unwrap(); // Execute init for bare repository

    // Simulate trying to reinitialize the bare repo
    let result = async {
        let args = InitArgs {
            bare: true,
            initial_branch: None,
            repo_directory: target_dir.to_str().unwrap().to_string(),
            quiet: false,
        };
        init(args).await
    };

    // Check for the error
    let err = result.await.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists); // Check error type
    assert!(err.to_string().contains("Initialization failed")); // Check error message contains "Already initialized"
}

#[tokio::test]
#[serial]
/// Test the init function with an initial branch name
async fn test_init_with_initial_branch() {
    // Set up the test environment without a Libra repository
    let temp_path = tempdir().unwrap();
    test::setup_clean_testing_env_in(temp_path.path());
    let _guard = test::ChangeDirGuard::new(temp_path.path());

    let args = InitArgs {
        bare: false,
        initial_branch: Some("main".to_string()),
        repo_directory: temp_path.path().to_str().unwrap().to_string(),
        quiet: false,
    };
    // Run the init function
    init(args).await.unwrap();

    // Verify the contents of the other directory
    verify_init(temp_path.path().join(".libra").as_path());

    // Verify the HEAD reference
    match Head::current().await {
        Head::Branch(current_branch) => {
            assert_eq!(current_branch, "main");
        }
        _ => panic!("should be branch"),
    };
}

#[tokio::test]
#[serial]
/// Test the init function with an invalid branch name
async fn test_init_with_invalid_branch() {
    // Cover all invalid branch name cases
    test_invalid_branch_name("master ").await;
    test_invalid_branch_name("master\t").await;
    test_invalid_branch_name("master\\").await;
    test_invalid_branch_name("master:").await;
    test_invalid_branch_name("master\"").await;
    test_invalid_branch_name("master?").await;
    test_invalid_branch_name("master*").await;
    test_invalid_branch_name("master[").await;
    test_invalid_branch_name("/master").await;
    test_invalid_branch_name("master/").await;
    test_invalid_branch_name("master.").await;
    test_invalid_branch_name("mast//er").await;
    test_invalid_branch_name("mast..er").await;
    test_invalid_branch_name("HEAD").await;
    test_invalid_branch_name("mast@{er").await;
    test_invalid_branch_name("").await;
    test_invalid_branch_name(".").await;
}

async fn test_invalid_branch_name(branch_name: &str) {
    let target_dir = tempdir().unwrap().into_path();
    let args = InitArgs {
        bare: false,
        initial_branch: Some(branch_name.to_string()),
        repo_directory: target_dir.to_str().unwrap().to_string(),
        quiet: false,
    };
    // Run the init function
    let result = init(args).await;
    // Check for the error
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput); // Check error type
    assert!(err.to_string().contains("invalid branch name")); // Check error message contains "invalid branch name"
}

#[tokio::test]
#[serial]
/// Test the init function with [directory] parameter
async fn test_init_with_directory() {
    let target_dir = tempdir().unwrap().into_path();

    // Create a test directory
    let test_dir = target_dir.join("test");

    let args = InitArgs {
        bare: false,
        initial_branch: None,
        repo_directory: test_dir.to_str().unwrap().to_owned(),
        quiet: false,
    };
    // Run the init function
    init(args).await.unwrap();

    // Verify that the `.libra` directory exists
    let libra_dir = test_dir.join(".libra");
    assert!(libra_dir.exists(), ".libra directory does not exist");

    // Verify the contents of the other directory
    verify_init(&libra_dir);
}

#[tokio::test]
#[serial]
/// Test the init function with invalid [directory] parameter
async fn test_init_with_invalid_directory() {
    let target_dir = tempdir().unwrap().into_path();

    // Create a test file instead of a directory
    let test_dir = target_dir.join("test.txt");

    // Create a file with the same name as the test directory
    fs::File::create(&test_dir).unwrap();

    let args = InitArgs {
        bare: false,
        initial_branch: None,
        repo_directory: test_dir.to_str().unwrap().to_owned(),
        quiet: false,
    };
    // Run the init function
    let result = init(args).await;

    // Check for the error
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput); // Check error type
    assert!(err
        .to_string()
        .contains("The target directory is not a directory")); // Check error message
}

#[tokio::test]
#[serial]
/// Tests that repository initialization fails when lacking write permissions in the target directory
async fn test_init_with_unauthorized_directory() {
    let target_dir = tempdir().unwrap().into_path();

    // Create a test directory
    let test_dir = target_dir.join("test");

    // Create a directory with restricted permissions
    fs::create_dir(&test_dir).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&test_dir, fs::Permissions::from_mode(0o444)).unwrap();
    }
    #[cfg(windows)]
    {
        let mut perms = fs::metadata(&test_dir).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&test_dir, perms).unwrap();
    }

    let args = InitArgs {
        bare: false,
        initial_branch: None,
        repo_directory: test_dir.to_str().unwrap().to_owned(),
        quiet: false,
    };
    // Run the init function
    let result = init(args).await;

    // Check for the error
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied); // Check error type
    assert!(err
        .to_string()
        .contains("The target directory is read-only")); // Check error message
}

#[tokio::test]
#[serial]
/// Test the init function with the --quiet flag by using --show-output
async fn test_init_quiet() {
    let target_dir = tempdir().unwrap().into_path();

    let args = InitArgs {
        bare: false,
        initial_branch: None,
        repo_directory: target_dir.to_str().unwrap().to_string(),
        quiet: true,
    };
    // Run the init function
    init(args).await.unwrap();

    // Verify that the `.libra` directory exists
    let libra_dir = target_dir.join(".libra");
    assert!(libra_dir.exists(), ".libra directory does not exist");

    // Verify the contents of the other directory
    verify_init(libra_dir.as_path());
}
