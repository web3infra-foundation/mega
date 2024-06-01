//! This module implements the `init` command for the Libra CLI.
//!
//!
//!
// Import necessary standard libraries
use std::{env, fs, io};

// Import necessary libraries from sea_orm
use sea_orm::{ActiveModelTrait, DbConn, DbErr, Set, TransactionTrait};

// Import necessary modules from the internal crate
use crate::internal::db;
use crate::internal::model::{config, reference};
use crate::utils::util::{DATABASE, ROOT_DIR};

/// Execute the init function
pub async fn execute() {
    init().await.unwrap();
}

/// Initialize a new Libra repository
/// This function creates the necessary directories and files for a new Libra repository.
/// It also sets up the database and the initial configuration.
#[allow(dead_code)]
pub async fn init() -> io::Result<()> {
    // Get the current directory
    let cur_dir = env::current_dir()?;
    // Join the current directory with the root directory
    let root_dir = cur_dir.join(ROOT_DIR);
    // Check if the root directory already exists
    if root_dir.exists() {
        println!("Already initialized - [{}]", root_dir.display());
        return Ok(());
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
    use super::init;
    use crate::utils::test;

    /// Test the init function
    #[tokio::test]
    async fn test_init() {
        // Set up the test environment without a Libra repository
        test::setup_clean_testing_env();

        // Run the init function
        init().await.unwrap();
    }
}