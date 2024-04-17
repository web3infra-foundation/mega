use std::path::PathBuf;
use std::{env, io};

pub const ROOT_DIR: &str = ".libra";
pub const DATABASE: &str = "libra.db";

pub fn cur_dir() -> PathBuf {
    env::current_dir().unwrap()
}

/// Try get the storage path of the repository, which is the path of the .libra directory
/// if the current directory is not a repository, return an error
pub fn try_get_storage_path() -> Result<PathBuf, io::Error> {
    /*递归获取储存库 */
    let mut current_dir = std::env::current_dir()?;
    loop {
        let mut git_path = current_dir.clone();
        git_path.push(ROOT_DIR);
        if git_path.exists() {
            return Ok(git_path);
        }
        if !current_dir.pop() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{:?} is not a git repository", std::env::current_dir()?),
            ));
        }
    }
}
/// Get the storage path of the repository
/// - panics if the current directory is not a repository
pub fn storage_path() -> PathBuf {
    try_get_storage_path().unwrap()
}

/// Get the working directory of the repository
/// - panics if the current directory is not a repository
pub fn working_dir() -> PathBuf {
    let mut storage_path = storage_path();
    storage_path.pop();
    storage_path
}

/// unify user input paths to relative paths with the repository root
/// panic if the path is not valid or not in the repository
pub fn pathspec_to_workpath(pathspec: Vec<String>) -> Vec<PathBuf> {
    let working_dir = working_dir();
    pathspec
        .into_iter()
        .map(|path| {
            let mut path = PathBuf::from(path);
            // relative path to absolute path
            if !path.is_absolute() {
                path = cur_dir().join(path);
            }

            // absolute path to relative path
            if let Ok(rel_path) = path.strip_prefix(&working_dir) {
                path = PathBuf::from(rel_path);
            } else {
                panic!("path {:?} is not in the repository", path);
            }
            path
        })
        .collect::<Vec<PathBuf>>()
}
