//! This module implements the `init` command for the Libra CLI.
//!
//! The `init` command creates a new Libra repository in the current directory or a specified directory.
//! It supports customizing the initial branch name with the `--initial-branch` parameter.
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

const DEFAULT_BRANCH: &str = "master";

#[derive(Parser, Debug, Clone)]
pub struct InitArgs {
    /// Create a bare repository
    #[clap(long, required = false)]
    pub bare: bool, // Default is false

    /// directory from which templates will be used
    #[clap(long = "template", name = "template-directory", required = false)]
    pub template: Option<String>,

    /// Set the initial branch name
    #[clap(short = 'b', long, required = false)]
    pub initial_branch: Option<String>,

    /// Create a repository in the specified directory
    #[clap(default_value = ".")]
    pub repo_directory: String,

    /// Suppress all output
    #[clap(long, short = 'q', required = false)]
    pub quiet: bool,

    /// Specify repository sharing mode
    ///
    /// Supported values:
    /// - `umask`: Default behavior (permissions depend on the user's umask).
    /// - `group`: Makes the repository group-writable so multiple users
    ///   in the same group can collaborate more easily.
    /// - `all`: Makes the repository readable by all users on the system.
    ///
    /// Note: On Windows, this option is ignored.
    #[clap(long, required = false, value_name = "MODE")]
    pub shared: Option<String>,
}

/// Execute the init function
pub async fn execute(args: InitArgs) {
    match init(args).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {e}");
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

/// Recursively copy the contents of the template directory to the destination directory.
///
/// # Behavior
/// - Directories are created as needed.
/// - Existing files in `dst` are NOT overwritten.
/// - Subdirectories are copied recursively.
fn copy_template(src: &Path, dst: &Path) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            fs::create_dir_all(&dest_path)?;
            copy_template(&entry.path(), &dest_path)?;
        } else if !dest_path.exists() {
            // Only copy if the file does not already exist
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

/// Apply repository with sharing mode
#[cfg(not(target_os = "windows"))]
fn apply_shared(root_dir: &Path, shared_mode: &str) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // Help function: recursively set permission bits for all files and dirs
    fn set_recursive(dir: &Path, mode: u32) -> io::Result<()> {
        for entry in walkdir::WalkDir::new(dir) {
            let entry = entry?;
            let path = entry.path();
            let metadata = fs::metadata(path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(mode);
            fs::set_permissions(path, perms)?;
        }
        Ok(())
    }
    // Match the shared_mode argument and apply permissions accordingly
    match shared_mode {
        "false" | "umask" => {} // default
        "true" | "group" => set_recursive(root_dir, 0o2775)?,
        "all" | "world" | "everybody" => set_recursive(root_dir, 0o2777)?,
        mode if mode.starts_with('0') && mode.len() == 4 => {
            if let Ok(bits) = u32::from_str_radix(&mode[1..], 8) {
                set_recursive(root_dir, bits)?;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid shared mode: {}", mode),
                ));
            }
        }
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid shared mode: {}", other),
            ));
        }
    }
    Ok(())
}

/// Only verify the shared_mode
#[cfg(target_os = "windows")]
fn apply_shared(root_dir: &Path, shared_mode: &str) -> io::Result<()> {
    match shared_mode {
        "true" | "false" | "umask" | "group" | "all" | "world" | "everybody" => {} // Valid string input
        mode if mode.starts_with('0') && mode.len() == 4 => {
            if let Ok(bits) = u32::from_str_radix(&mode[1..], 8) { //Valid perm input
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid shared mode: {}", mode),
                ));
            }
        }
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid shared mode: {}", other),
            ));
        }
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
    if let Some(ref branch_name) = args.initial_branch
        && !branch::is_valid_git_branch_name(branch_name)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "invalid branch name: '{branch_name}'.\n\nBranch names must:\n- Not contain spaces, control characters, or any of these characters: \\ : \" ? * [\n- Not start or end with a slash ('/'), or end with a dot ('.')\n- Not contain consecutive slashes ('//') or dots ('..')\n- Not be reserved names like 'HEAD' or contain '@{{'\n- Not be empty or just a dot ('.')\n\nPlease choose a valid branch name."
            ),
        ));
    }

    // Check if the target directory is writable
    match is_writable(&cur_dir) {
        Ok(_) => {}
        Err(e) => {
            return Err(e);
        }
    }

    // ensure root dir exists
    fs::create_dir_all(&root_dir)?;

    // If a template path is provided, copy the template files to the root directory
    if let Some(template_path) = &args.template {
        let template_dir = Path::new(template_path);
        if template_dir.exists() {
            copy_template(template_dir, &root_dir)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("template directory '{}' does not exist", template_path),
            ));
        }
    } else {
        // Create info & hooks
        let dirs = ["info", "hooks"];
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
        // Create .libra/hooks/pre-commit.sh
        fs::write(
            root_dir.join("hooks").join("pre-commit.sh"),
            include_str!("../../template/pre-commit.sh"),
        )?;

        // Set Permission
        #[cfg(not(target_os = "windows"))]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(root_dir.join("hooks").join("pre-commit.sh"), perms)?;
        }

        // Create .libra/hooks/pre-commit.ps1
        fs::write(
            root_dir.join("hooks").join("pre-commit.ps1"),
            include_str!("../../template/pre-commit.ps1"),
        )?;
    }

    // Complete .libra and sub-directories
    let dirs = ["objects/pack", "objects/info"];
    for dir in dirs {
        fs::create_dir_all(root_dir.join(dir))?;
    }

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

    // Create config table with bare parameter consideration
    init_config(&conn, args.bare).await.unwrap();

    // Determine the initial branch name: use provided name or default to "main"
    let initial_branch_name = args
        .initial_branch
        .unwrap_or_else(|| DEFAULT_BRANCH.to_owned());

    // Create HEAD
    reference::ActiveModel {
        name: Set(Some(initial_branch_name.clone())),
        kind: Set(reference::ConfigKind::Head),
        ..Default::default() // all others are `NotSet`
    }
    .insert(&conn)
    .await
    .unwrap();

    // Set .libra as hidden
    set_dir_hidden(root_dir.to_str().unwrap())?;

    // Apply shared permissions if requested
    if let Some(shared_mode) = &args.shared {
        apply_shared(&root_dir, shared_mode)?;
    }

    if !args.quiet {
        let repo_type = if args.bare { "bare " } else { "" };
        println!(
            "Initializing empty {repo_type}Libra repository in {} with initial branch '{initial_branch_name}'",
            root_dir.display()
        );
    }

    Ok(())
}

/// Initialize the configuration for the Libra repository
/// This function creates the necessary configuration entries in the database.
async fn init_config(conn: &DbConn, is_bare: bool) -> Result<(), DbErr> {
    // Begin a new transaction
    let txn = conn.begin().await?;

    // Define the configuration entries for non-Windows systems
    #[cfg(not(target_os = "windows"))]
    let entries = [
        ("repositoryformatversion", "0"),
        ("filemode", "true"),
        ("bare", if is_bare { "true" } else { "false" }),
        ("logallrefupdates", "true"),
    ];

    // Define the configuration entries for Windows systems
    #[cfg(target_os = "windows")]
    let entries = [
        ("repositoryformatversion", "0"),
        ("filemode", "false"), // no filemode on windows
        ("bare", if is_bare { "true" } else { "false" }),
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
    Command::new("attrib").arg("+H").arg(dir).spawn()?.wait()?; // Wait for command execution to complete
    Ok(())
}

/// On Unix-like systems, directories starting with a dot are hidden by default
/// Therefore, this function does nothing.
#[cfg(not(target_os = "windows"))]
fn set_dir_hidden(_dir: &str) -> io::Result<()> {
    // on unix-like systems, dotfiles are hidden by default
    Ok(())
}
