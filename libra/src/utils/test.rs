//! This module contains the tests util functions for the Libra.
//!
//!
//!
#![cfg(test)]

use std::io::Write;
use std::path::Path;
use std::{env, fs, path::PathBuf};

use crate::command;
use crate::utils::util;

pub const TEST_DIR: &str = "libra_test_repo";

fn find_cargo_dir() -> PathBuf {
    let cargo_path = env::var("CARGO_MANIFEST_DIR");

    match cargo_path {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            // vscode DEBUG test does not have the CARGO_MANIFEST_DIR macro, manually try to find cargo.toml
            println!("CARGO_MANIFEST_DIR not found, try to find Cargo.toml manually");
            let mut path = util::cur_dir();

            loop {
                path.push("Cargo.toml");
                if path.exists() {
                    break;
                }
                if !path.pop() {
                    panic!("Could not find CARGO_MANIFEST_DIR");
                }
            }

            path.pop();

            path
        }
    }
}

/// Sets up the environment for testing.
///
/// This function performs the following steps:
/// 1. Installs the color_backtrace crate to provide colored backtraces.
/// 2. Finds the directory where the Cargo.toml file is located.
/// 3. Appends the test directory to the Cargo directory.
/// 4. If the test directory does not exist, it creates it.
/// 5. Sets the current directory to the test directory.
fn setup_env() {
    // Install the color_backtrace crate to provide colored backtraces
    color_backtrace::install();

    // Find the directory where the Cargo.toml file is located
    let mut path = find_cargo_dir();

    // Append the test directory to the Cargo directory
    path.push(TEST_DIR);

    // If the test directory does not exist, create it
    if !path.exists() {
        fs::create_dir(&path).unwrap();
    }

    // Set the current directory to the test directory
    env::set_current_dir(&path).unwrap();
}

/// Sets up a clean environment for testing.
///
/// This function first calls `setup_env()` to switch the current directory to the test directory.
/// Then, it checks if the Libra root directory (`.libra`) exists in the current directory.
/// If it does, the function removes the entire `.libra` directory.
pub fn setup_clean_testing_env() {
    // Switch the current directory to the test directory
    setup_env();

    // Get the current directory
    let cur_path = util::cur_dir();

    // Append the Libra root directory to the current directory
    let root_path = cur_path.join(util::ROOT_DIR);

    // If the Libra root directory exists, remove it
    if root_path.exists() {
        fs::remove_dir_all(&root_path).unwrap();
    }

    // Define the directories that are present in a bare repository
    let bare_repo_dirs = ["objects", "info", "description", "libra.db"];

    // Remove the directories that are present in a bare repository if they exist
    for dir in bare_repo_dirs.iter() {
        let bare_repo_path = cur_path.join(dir);
        if bare_repo_path.exists() && bare_repo_path.is_dir() {
            fs::remove_dir_all(&bare_repo_path).unwrap();
        } else if bare_repo_path.exists() && !bare_repo_path.is_dir() {
            // Remove the file if it exists
            fs::remove_file(&bare_repo_path).unwrap();
        }
    }
}

/// switch to test dir and create a new .libra
pub async fn setup_with_new_libra() {
    setup_clean_testing_env();
    let args = command::init::InitArgs {
        bare: false,
        initial_branch: None,
        repo_directory: util::cur_dir().to_str().unwrap().to_string(),
        quiet: false,
    };
    command::init::init(args).await.unwrap();
}

pub fn init_debug_logger() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init(); // avoid multi-init
}

pub fn init_logger() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init(); // avoid multi-init
}

/// create file related to working directory
pub fn ensure_file(path: impl AsRef<Path>, content: Option<&str>) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap(); // ensure父目录
    let mut file = fs::File::create(util::working_dir().join(path))
        .unwrap_or_else(|_| panic!("Cannot create file：{:?}", path));
    if let Some(content) = content {
        file.write_all(content.as_bytes()).unwrap();
    } else {
        // write filename if no content
        file.write_all(path.file_name().unwrap().as_encoded_bytes())
            .unwrap();
    }
}

pub fn reset_working_dir() {
    env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();
}
