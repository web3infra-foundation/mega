//! This module implements the `init` command for the Libra CLI.
//!
//!
//!
use std::{fs, io::{self, ErrorKind}, path::Path};

use sea_orm::{ActiveModelTrait, DbConn, DbErr, Set, TransactionTrait};

use clap::Parser;

use crate::internal::db;
use crate::internal::model::{config, reference};
use crate::utils::util::{DATABASE, ROOT_DIR};

#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Create a bare repository
    #[clap(short, long, required = false)]
    pub bare: bool,  // Default is false

    /// Create a repository in the specified directory
    #[clap(default_value = ".")]
    pub repo_directory: String,
}

/// Execute the init function
pub async fn execute(args: InitArgs){
   match init(args).await{
    Ok(_) => {},
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
    let root_dir = if args.bare{
        cur_dir.clone()
    }else{
        cur_dir.join(ROOT_DIR)
    };

    // Check if the root directory already exists
    if is_reinit(&cur_dir) {
        println!("Already initialized - [{}]", root_dir.display());
        
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Initialization failed: The repository is already initialized at the specified location. If you wish to reinitialize, please remove the existing directory or file.",
        ));
           
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
    let database = root_dir.join(DATABASE);
    let conn = db::create_database(database.to_str().unwrap()).await?;

    // Create config table
    init_config(&conn).await.unwrap();

    // Create HEAD
    reference::ActiveModel {
        name: Set(Some("master".to_owned())),
        kind: Set(reference::ConfigKind::Head),
        ..Default::default() // all others are `NotSet`
    }
        .insert(&conn)
        .await
        .unwrap();
    
    
    // Set .libra as hidden
    set_dir_hidden(root_dir.to_str().unwrap())?;
    println!(
        "Initializing empty Libra repository in {}",
        root_dir.display()
    );
      
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
    use std::{env, os::unix::fs::PermissionsExt};
    use super::*;
    use crate::utils::test;

    pub fn verify_init(base_dir: &Path){

        // List of subdirectories to verify
        let dirs = ["objects/pack", "objects/info", "info"];

        // Loop through the directories and verify they exist
        for dir in dirs {
            let dir_path = base_dir.join(dir);
            assert!(dir_path.exists(), "Directory {} does not exist", dir);
        }

        // Additional file verification
        let files = [
            "description",
            "libra.db",
            "info/exclude",
        ];

        for file in files {
            let file_path = base_dir.join(file);
            assert!(file_path.exists(), "File {} does not exist", file);
        }
    }
    /// Test the init function with no parameters
    #[tokio::test]
    async fn test_init() {
        // Set up the test environment without a Libra repository
        test::setup_clean_testing_env();
        let cur_dir = env::current_dir().unwrap();
        let args = InitArgs {bare: false, repo_directory: cur_dir.to_str().unwrap().to_owned() };
        // Run the init function
        init(args).await.unwrap();

        // Verify that the `.libra` directory exists
        let libra_dir = Path::new(".libra");
        assert!(libra_dir.exists(), ".libra directory does not exist");

        // Verify the contents of the other directory
        verify_init(libra_dir);
    }

    /// Test the init function with the --bare flag       
    #[tokio::test]
    async fn test_init_bare() {
        // Set up the test environment without a Libra repository
        test::setup_clean_testing_env();
        // Run the init function with --bare flag
        let cur_dir = env::current_dir().unwrap();
        let args = InitArgs {bare: true, repo_directory: cur_dir.to_str().unwrap().to_owned() };
        // Run the init function
        init(args).await.unwrap();

        let libra_dir = Path::new(".");
        // Verify the contents of the other directory
        verify_init(libra_dir);
    }
    /// Test the init function with the --bare flag and an existing repository    
    #[tokio::test]
    async fn test_init_bare_with_existing_repo() {
        // Set up the test environment for a bare repository
        test::setup_clean_testing_env();

        // Initialize a bare repository
        let cur_dir = env::current_dir().unwrap();
        let init_args = InitArgs { bare: false, repo_directory: cur_dir.to_str().unwrap().to_owned() };
        init(init_args).await.unwrap(); // Execute init for bare repository
    
        // Simulate trying to reinitialize the bare repo
        let result = async {
            let args = InitArgs { bare: true, repo_directory: cur_dir.to_str().unwrap().to_owned() };
            init(args).await
        };

        // Check for the error
        let err = result.await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);  // Check error type
        assert!(err.to_string().contains("Initialization failed"));  // Check error message contains "Already initialized"
    }

    /// Test the init function with [directory] parameter
    #[tokio::test]
    async fn test_init_with_directory() {
        // Set up the test environment without a Libra repository
        test::setup_clean_testing_env();

        // Create a test directory
        let cur_dir = env::current_dir().unwrap();
        let test_dir = cur_dir.join("test");

        let args = InitArgs {bare: false, repo_directory: test_dir.to_str().unwrap().to_owned() };
        // Run the init function
        init(args).await.unwrap();

        // Verify that the `.libra` directory exists
        let libra_dir = test_dir.join(".libra");
        assert!(libra_dir.exists(), ".libra directory does not exist");

        // Verify the contents of the other directory
        verify_init(&libra_dir);
    }

    /// Test the init function with invalid [directory] parameter
    #[tokio::test]
    async fn test_init_with_invalid_directory() {
        // Set up the test environment without a Libra repository
        test::setup_clean_testing_env();

        // Create a test file instead of a directory
        let cur_dir = env::current_dir().unwrap();
        let test_dir = cur_dir.join("test.txt");

        // Create a file with the same name as the test directory
        fs::File::create(&test_dir).unwrap();

        let args = InitArgs {bare: false, repo_directory: test_dir.to_str().unwrap().to_owned() };
        // Run the init function
        let result = init(args).await;

        // Check for the error
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);  // Check error type
        assert!(err.to_string().contains("The target directory is not a directory"));  // Check error message
    }

    #[tokio::test]
    async fn test_init_with_unauthorized_directory() {
        // Set up the test environment without a Libra repository
        test::setup_clean_testing_env();

        // Create a test directory
        let cur_dir = env::current_dir().unwrap();
        let test_dir = cur_dir.join("test");

        // Create a directory with restricted permissions
        fs::create_dir(&test_dir).unwrap();
        fs::set_permissions(&test_dir, fs::Permissions::from_mode(0o444)).unwrap();

        let args = InitArgs {bare: false, repo_directory: test_dir.to_str().unwrap().to_owned() };
        // Run the init function
        let result = init(args).await;

        // Check for the error
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);  // Check error type
        assert!(err.to_string().contains("The target directory is read-only"));  // Check error message
    }

}