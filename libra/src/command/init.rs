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

    /// Separate git dir from working tree
    #[clap(long, required = false)]
    pub separate_git_dir: Option<String>,
}

/// Execute the init function
pub async fn execute(args: InitArgs) {
    let quiet = args.quiet; // Store quiet flag before moving args
    match init(args).await {
        Ok(_) => {}
        Err(e) => {
            if !quiet {
                eprintln!("Error: {}", e);
            }
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
    
    // Handle separate git dir
    let (root_dir, git_link_file) = if let Some(ref separate_git_dir) = args.separate_git_dir {
        let separate_path = Path::new(separate_git_dir).to_path_buf();
        let git_link_file = if args.bare {
            None
        } else {
            Some(cur_dir.join(".libra"))
        };
        (separate_path, git_link_file)
    } else {
        // Join the current directory with the root directory
        let root_dir = if args.bare {
            cur_dir.clone()
        } else {
            cur_dir.join(ROOT_DIR)
        };
        (root_dir, None)
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
    
    // Create git link file if using separate git dir
    if let Some(git_link_file) = git_link_file {
        let git_link_content = format!("gitdir: {}", root_dir.display());
        fs::write(&git_link_file, git_link_content)?;
        
        // Set the git link file as hidden too
        set_dir_hidden(git_link_file.to_str().unwrap())?;
    }
    
    if !args.quiet {
        if args.separate_git_dir.is_some() {
            println!(
                "Initializing empty Libra repository in {} (separate git dir: {})",
                cur_dir.display(),
                root_dir.display()
            );
        } else {
            println!(
                "Initializing empty Libra repository in {}",
                root_dir.display()
            );
        }
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
    use clap::Parser;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_init_args_parsing() {
        // Test normal parsing
        let args = InitArgs::try_parse_from(&["libra", "init", "test_dir"]).unwrap();
        assert_eq!(args.repo_directory, "test_dir");
        assert!(!args.bare);
        assert!(!args.quiet);
        assert!(args.initial_branch.is_none());

        // Test with --quiet flag
        let args = InitArgs::try_parse_from(&["libra", "init", "--quiet", "test_dir"]).unwrap();
        assert!(args.quiet);
        assert_eq!(args.repo_directory, "test_dir");

        // Test with -q flag
        let args = InitArgs::try_parse_from(&["libra", "init", "-q", "test_dir"]).unwrap();
        assert!(args.quiet);

        // Test with --bare and --quiet
        let args = InitArgs::try_parse_from(&["libra", "init", "--bare", "--quiet", "test_dir"]).unwrap();
        assert!(args.bare);
        assert!(args.quiet);

        // Test with --initial-branch
        let args = InitArgs::try_parse_from(&["libra", "init", "-b", "main", "--quiet"]).unwrap();
        assert_eq!(args.initial_branch, Some("main".to_string()));
        assert!(args.quiet);

        // Test with --separate-git-dir
        let args = InitArgs::try_parse_from(&["libra", "init", "--separate-git-dir", "/tmp/git", "test_dir"]).unwrap();
        assert_eq!(args.separate_git_dir, Some("/tmp/git".to_string()));
        assert_eq!(args.repo_directory, "test_dir");
    }

    #[tokio::test]
    async fn test_init_quiet_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_repo");
        
        // Test normal initialization
        let args = InitArgs {
            bare: false,
            initial_branch: None,
            repo_directory: test_path.to_str().unwrap().to_string(),
            quiet: false,
            separate_git_dir: None,
        };
        
        // This should succeed without panicking
        let result = init(args).await;
        assert!(result.is_ok(), "Normal init should succeed");
        
        // Verify repository structure was created
        assert!(test_path.join(".libra").exists());
        assert!(test_path.join(".libra/objects").exists());
        assert!(test_path.join(".libra/description").exists());
    }

    #[tokio::test]
    async fn test_init_quiet_on_existing_repo() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_repo");
        
        // First initialization
        let args1 = InitArgs {
            bare: false,
            initial_branch: None,
            repo_directory: test_path.to_str().unwrap().to_string(),
            quiet: true,
            separate_git_dir: None,
        };
        
        let result1 = init(args1).await;
        assert!(result1.is_ok());
        
        // Second initialization (should fail but quietly)
        let args2 = InitArgs {
            bare: false,
            initial_branch: None,
            repo_directory: test_path.to_str().unwrap().to_string(),
            quiet: true,
            separate_git_dir: None,
        };
        
        let result2 = init(args2).await;
        assert!(result2.is_err());
        assert_eq!(result2.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn test_is_reinit() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path();
        
        // Should return false for empty directory
        assert!(!is_reinit(test_path));
        
        // Create .libra/description file
        fs::create_dir_all(test_path.join(".libra")).unwrap();
        fs::write(test_path.join(".libra/description"), "test").unwrap();
        
        // Should return true now
        assert!(is_reinit(test_path));
    }

    #[test]
    fn test_is_writable() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path();
        
        // Should be writable
        assert!(is_writable(test_path).is_ok());
    }

    #[tokio::test]
    async fn test_init_separate_git_dir() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join("work");
        let git_dir = temp_dir.path().join("git");
        
        // Create work directory
        fs::create_dir_all(&work_dir).unwrap();

        let args = InitArgs {
            bare: false,
            initial_branch: None,
            repo_directory: work_dir.to_str().unwrap().to_string(),
            quiet: true,
            separate_git_dir: Some(git_dir.to_str().unwrap().to_string()),
        };

        // This should succeed
        let result = init(args).await;
        assert!(result.is_ok(), "Init with separate git dir should succeed");

        // Check that the git directory was created at the separate location
        assert!(git_dir.exists(), "Separate git directory should exist");
        assert!(git_dir.join("objects").exists(), "Git objects directory should exist");
        assert!(git_dir.join("description").exists(), "Git description file should exist");

        // Check that a .libra file (not directory) exists in the work directory
        let libra_file = work_dir.join(".libra");
        assert!(libra_file.exists(), ".libra file should exist in work directory");
        assert!(libra_file.is_file(), ".libra should be a file, not a directory");

        // Check the content of the .libra file
        let content = fs::read_to_string(&libra_file).unwrap();
        assert!(content.starts_with("gitdir: "), ".libra file should start with 'gitdir: '");
        assert!(content.contains(&git_dir.to_string_lossy().to_string()), 
                ".libra file should contain path to separate git dir");
    }

    #[tokio::test]
    async fn test_init_separate_git_dir_bare() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join("work");
        let git_dir = temp_dir.path().join("git");
        
        // Create work directory
        fs::create_dir_all(&work_dir).unwrap();

        let args = InitArgs {
            bare: true,
            initial_branch: None,
            repo_directory: work_dir.to_str().unwrap().to_string(),
            quiet: true,
            separate_git_dir: Some(git_dir.to_str().unwrap().to_string()),
        };

        // This should succeed
        let result = init(args).await;
        assert!(result.is_ok(), "Bare init with separate git dir should succeed");

        // Check that the git directory was created at the separate location
        assert!(git_dir.exists(), "Separate git directory should exist");
        assert!(git_dir.join("objects").exists(), "Git objects directory should exist");

        // For bare repositories with separate git dir, no .libra file should be created
        let libra_file = work_dir.join(".libra");
        assert!(!libra_file.exists(), ".libra file should not exist in bare repository");
    }
}