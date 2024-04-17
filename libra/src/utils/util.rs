use std::path::{Path, PathBuf};
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

/// Get the working directory of the repository as a string, panics if the path is not valid utf-8
pub fn working_dir_string() -> String {
    working_dir().to_str().unwrap().to_string()
}

/// clean up the path
/// didn't use `canonicalize` because path may not exist in file system but in the repository
fn simplify_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();

    // 迭代路径中的每个组件
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                // 对于 `..`，尝试移除最后一个路径组件
                result.pop();
            },
            std::path::Component::CurDir => {
                // 对于 `.`，不做任何操作，继续
                continue;
            },
            // 直接添加其它类型的组件到结果路径中
            _ => result.push(component.as_os_str()),
        }
    }

    result
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

            // clean up the path
            path = simplify_path(&path);
            
            // absolute path to relative path
            if let Ok(rel_path) = path.strip_prefix(&working_dir) {
                path = PathBuf::from(rel_path);
            } else {
                panic!("fatal: path {:?} is not in the repository", path);
            }
            path
        })
        .collect::<Vec<PathBuf>>()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::test;
    #[tokio::test]
    async fn test_pathspec_to_workpath_with_workdir() {
        test::setup_with_new_libra().await;
        let path = pathspec_to_workpath(vec!["test.rs".to_owned(), working_dir_string()]);
        path.iter().for_each(|p| {
            assert!(p.is_relative());
            // all path should be relative to the working directory
            assert!(p.starts_with(PathBuf::from("")));
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn test_pathspec_to_workpath_with_outside_path() {
        test::setup_with_new_libra().await;
        let _ = pathspec_to_workpath(vec![
            "test.rs".to_owned(),
            working_dir().join("../test").to_str().unwrap().to_owned(),
        ]);
    }
}
