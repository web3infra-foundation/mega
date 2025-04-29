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

pub struct ChangeDirGuard {
    old_dir: PathBuf,
}

impl ChangeDirGuard {
    /// Creates a new `ChangeDirGuard` that changes the current directory to `new_dir`.
    /// This will automatically change the directory back to the original one when the guard is dropped.
    ///
    /// However, it **MUST** be used in a single-threaded context.
    ///
    /// # Arguments
    ///
    /// * `new_dir` - The new directory to change to.
    ///
    /// # Returns
    ///
    /// * A `ChangeDirGuard` instance that will change the directory back to the original one when dropped.
    ///
    pub fn new(new_dir: impl AsRef<Path>) -> Self {
        let old_dir = env::current_dir().unwrap();
        env::set_current_dir(new_dir).unwrap();
        Self { old_dir }
    }
}

impl Drop for ChangeDirGuard {
    fn drop(&mut self) {
        env::set_current_dir(&self.old_dir).unwrap();
    }
}

pub fn find_cargo_dir() -> PathBuf {
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

/// Sets up a clean environment for testing.
///
/// This function first calls `setup_env()` to switch the current directory to the test directory.
/// Then, it checks if the Libra root directory (`.libra`) exists in the current directory.
/// If it does, the function removes the entire `.libra` directory.
pub fn setup_clean_testing_env_in(temp_path: impl AsRef<Path>) {
    assert!(temp_path.as_ref().exists(), "temp_path does not exist");
    assert!(temp_path.as_ref().is_dir(), "temp_path is not a directory");
    assert!(
        temp_path.as_ref().read_dir().unwrap().count() == 0,
        "temp_path is not empty"
    );

    tracing::info!("Using libra testing path: {:?}", temp_path.as_ref());

    // Define the directories that are present in a bare repository
    let owned = temp_path.as_ref().to_path_buf();
    let bare_repo_dirs = ["objects", "info", "description", "libra.db"];

    // Remove the directories that are present in a bare repository if they exist
    for dir in bare_repo_dirs.iter() {
        let bare_repo_path = owned.join(dir);
        if bare_repo_path.exists() && bare_repo_path.is_dir() {
            fs::remove_dir_all(&bare_repo_path).unwrap();
        } else if bare_repo_path.exists() && !bare_repo_path.is_dir() {
            // Remove the file if it exists
            fs::remove_file(&bare_repo_path).unwrap();
        }
    }
}

/// switch to test dir and create a new .libra
pub async fn setup_with_new_libra_in(temp_path: impl AsRef<Path>) {
    setup_clean_testing_env_in(temp_path.as_ref());
    let args = command::init::InitArgs {
        bare: false,
        initial_branch: None,
        repo_directory: temp_path.as_ref().to_str().unwrap().to_string(),
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

/// reset working directory to the root of the module
pub fn reset_working_dir() {
    env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();
}
