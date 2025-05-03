//! This module implements the `init` command for the Libra CLI.
//!
//!
//!
use std::{
    fs,
    io::{self, ErrorKind},
    path::Path,
};

use sea_orm::{ActiveModelTrait, DbConn, DbErr, Set, TransactionTrait};

use clap::Parser;

use crate::command::branch;
use crate::internal::db;
use crate::internal::model::{config, reference};
use crate::utils::util::{DATABASE, ROOT_DIR};

#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Create a bare repository
    #[clap(long, required = false)]
    pub bare: bool, // Default is false

    /// Set the initial branch name
    #[clap(short = 'b', long, required = false)]
    pub initial_branch: Option<String>,

    /// Create a repository in the specified directory
    #[clap(default_value = ".")]
    pub repo_directory: String,

    /// Suppress all output
    #[clap(long, short = 'q', required = false)]
    pub quiet: bool,
}

/// Execute the init function
pub async fn execute(args: InitArgs) {
    match init(args).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Check if the repository has already been initialized based on the presence of the description file.
fn is_reinit(cur_dir: &Path) -> bool {
    let bare_head_path = cur_dir.join("description");
    let head_path = cur_dir.join(".libra/description");
    // Check the presence of the description file
    head_path.exists() || bare_head_path.exists()
}

/// Check if the target directory is writable
fn is_writable(cur_dir: &Path) -> io::Result<()> {
    match fs::metadata(cur_dir) {
        Ok(metadata) => {
            // Check if the target directory is a directory
            if !metadata.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "The target directory is not a directory.",
                ));
            }
            // Check permissions
            if metadata.permissions().readonly() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "The target directory is read-only.",
                ));
            }
        }
        Err(e) if e.kind() != ErrorKind::NotFound => {
            return Err(e);
        }
        _ => {}
    }
    Ok(())
}

/// Initialize a new Libra repository
/// This function creates the necessary directories and files for a new Libra repository.
/// It also sets up the database and the initial configuration.
#[allow(dead_code)]
pub async fn init(args: InitArgs) -> io::Result<()> {
    // Get the current directory
    // let cur_dir = env::current_dir()?;
    let cur_dir = Path::new(&args.repo_directory).to_path_buf();
    // Join the current directory with the root directory
    let root_dir = if args.bare {
        cur_dir.clone()
    } else {
        cur_dir.join(ROOT_DIR)
    };

    // Check if the root directory already exists
    if is_reinit(&cur_dir) {
        if !args.quiet {
            eprintln!("Already initialized - [{}]", root_dir.display());
        }
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Initialization failed: The repository is already initialized at the specified location.
            If you wish to reinitialize, please remove the existing directory or file.",
        ));
    }

    // Check if the branch name is valid
    if let Some(ref branch_name) = args.initial_branch {
        if !branch::is_valid_git_branch_name(branch_name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid branch name: '{}'.\n\nBranch names must:\n- Not contain spaces, control characters, or any of these characters: \\ : \" ? * [\n- Not start or end with a slash ('/'), or end with a dot ('.')\n- Not contain consecutive slashes ('//') or dots ('..')\n- Not be reserved names like 'HEAD' or contain '@{{'\n- Not be empty or just a dot ('.')\n\nPlease choose a valid branch name.", branch_name),
            ));
        }
    }

    // Check if the target directory is writable
    match is_writable(&cur_dir) {
        Ok(_) => {}
        Err(e) => {
            return Err(e);
        }
    }

    // Create .libra & sub-dirs
    let dirs = ["objects/pack", "objects/info", "info"];
    for dir in dirs {
        fs::create_dir_all(root_dir.join(dir))?;
    }
    // Create info/exclude
    // `include_str!` includes the file content while compiling
    fs::write(
        root_dir.join("info/exclude"),
        include_str!("../../template/exclude"),
    )?;
    // Create .libra/description
    fs::write(
        root_dir.join("description"),
        include_str!("../../template/description"),
    )?;

    // Create database: .libra/libra.db
    let conn;
    let database = root_dir.join(DATABASE);

    #[cfg(target_os = "windows")]
    {
        // On Windows, we need to convert the path to a UNC path
        let database = database.to_str().unwrap().replace("\\", "/");
        conn = db::create_database(database.as_str()).await?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Unix-like systems, we do no more
        conn = db::create_database(database.to_str().unwrap()).await?;
    }

    // Create config table
    init_config(&conn).await.unwrap();

    // Create HEAD
    reference::ActiveModel {
        name: Set(Some(
            args.initial_branch.unwrap_or_else(|| "master".to_owned()),
        )),
        kind: Set(reference::ConfigKind::Head),
        ..Default::default() // all others are `NotSet`
    }
    .insert(&conn)
    .await
    .unwrap();

    // Set .libra as hidden
    set_dir_hidden(root_dir.to_str().unwrap())?;
    if !args.quiet {
        println!(
            "Initializing empty Libra repository in {}",
            root_dir.display()
        );
    }

    Ok(())
}
/// Initialize the configuration for the Libra repository
/// This function creates the necessary configuration entries in the database.
async fn init_config(conn: &DbConn) -> Result<(), DbErr> {
    // Begin a new transaction
    let txn = conn.begin().await?;

    // Define the configuration entries for non-Windows systems
    #[cfg(not(target_os = "windows"))]
    let entries = [
        ("repositoryformatversion", "0"),
        ("filemode", "true"),
        ("bare", "false"),
        ("logallrefupdates", "true"),
    ];

    // Define the configuration entries for Windows systems
    #[cfg(target_os = "windows")]
    let entries = [
        ("repositoryformatversion", "0"),
        ("filemode", "false"), // no filemode on windows
        ("bare", "false"),
        ("logallrefupdates", "true"),
        ("symlinks", "false"),  // no symlinks on windows
        ("ignorecase", "true"), // ignorecase on windows
    ];

    // Insert each configuration entry into the database
    for (key, value) in entries {
        // tip: Set(None) == NotSet == default == NULL
        let entry = config::ActiveModel {
            configuration: Set("core".to_owned()),
            key: Set(key.to_owned()),
            value: Set(value.to_owned()),
            ..Default::default() // id & name NotSet
        };
        entry.insert(&txn).await?;
    }
    // Commit the transaction
    txn.commit().await?;
    Ok(())
}

/// Set a directory as hidden on Windows systems
/// This function uses the `attrib` command to set the directory as hidden.
#[cfg(target_os = "windows")]
fn set_dir_hidden(dir: &str) -> io::Result<()> {
    use std::process::Command;
    Command::new("attrib").arg("+H").arg(dir).spawn()?.wait()?; // 等待命令执行完成
    Ok(())
}

/// On Unix-like systems, directories starting with a dot are hidden by default
/// Therefore, this function does nothing.
#[cfg(not(target_os = "windows"))]
fn set_dir_hidden(_dir: &str) -> io::Result<()> {
    // on unix-like systems, dotfiles are hidden by default
    Ok(())
}

/// Unit tests for the init module
#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::head::Head;
    use crate::utils::test;
    use serial_test::serial;
    use tempfile::tempdir;

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
}
