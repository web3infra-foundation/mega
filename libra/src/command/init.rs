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
}

/// Execute the init function
pub async fn execute(args: InitArgs) {
    match init(args).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {e}");
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

/// Recursively copy the contents of the template directory to the destination directory
fn copy_template(src: &Path, dest: &Path) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if path.is_dir() {
            fs::create_dir_all(&dest_path)?;
            copy_template(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
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
    if let Some(ref branch_name) = args.initial_branch {
        if !branch::is_valid_git_branch_name(branch_name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid branch name: '{branch_name}'.\n\nBranch names must:\n- Not contain spaces, control characters, or any of these characters: \\ : \" ? * [\n- Not start or end with a slash ('/'), or end with a dot ('.')\n- Not contain consecutive slashes ('//') or dots ('..')\n- Not be reserved names like 'HEAD' or contain '@{{'\n- Not be empty or just a dot ('.')\n\nPlease choose a valid branch name."),
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
    let dirs = ["objects/pack", "objects/info", "info", "hooks"];
    for dir in dirs {
        fs::create_dir_all(root_dir.join(dir))?;
    }

    if let Some(template_path) = &args.template {
        let template_dir = Path::new(template_path);
        if !template_dir.is_dir() {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                format!("template directory '{}' not found", template_path),
            ));
        }
        copy_template(template_dir, &root_dir)?;
    } else {
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
mod tests {}
