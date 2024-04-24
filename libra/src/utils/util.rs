use std::path::{Path, PathBuf};
use std::{env, fs, io};
use std::io::{BufReader, Read};
use path_abs::{PathAbs, PathInfo};
use sha1::{Digest, Sha1};
use storage::driver::file_storage::local_storage::LocalStorage;
use venus::hash::SHA1;
use crate::utils::path;

pub const ROOT_DIR: &str = ".libra";
pub const DATABASE: &str = "libra.db";

pub fn cur_dir() -> PathBuf {
    env::current_dir().unwrap()
}

/// Try get the storage path of the repository, which is the path of the `.libra` directory
/// - if the current directory is not a repository, return an error
pub fn try_get_storage_path() -> Result<PathBuf, io::Error> {
    /*递归获取储存库 */
    let mut cur_dir = env::current_dir()?;
    loop {
        let mut libra = cur_dir.clone();
        libra.push(ROOT_DIR);
        if libra.exists() {
            return Ok(libra);
        }
        if !cur_dir.pop() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{:?} is not a git repository", env::current_dir()?),
            ));
        }
    }
}
/// Get the storage path of the repository, aka `.libra`
/// - panics if the current directory is not a repository
pub fn storage_path() -> PathBuf {
    try_get_storage_path().unwrap()
}
/// Check if libra repo exists
pub fn check_repo_exist() -> bool {
    if try_get_storage_path().is_err() {
        eprintln!("fatal: not a libra repository (or any of the parent directories): .libra");
        return false;
    }
    true
}

/// Get `LocalStorage` for the `objects` directory
pub fn objects_storage() -> LocalStorage {
    LocalStorage::init(path::objects())
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

/// Turn a path to a relative path to the working directory
/// - not check existence
pub fn to_workdir_path(path: impl AsRef<Path>) -> PathBuf {
    to_relative(path, working_dir())
}

/// Turn a workdir path to absolute path
pub fn workdir_to_absolute(path: impl AsRef<Path>) -> PathBuf {
    working_dir().join(path.as_ref())
}

pub fn to_relative<P, B>(path: P, base: B) -> PathBuf
where P: AsRef<Path>, B: AsRef<Path>
{
    let path_abs = PathAbs::new(path.as_ref()).unwrap(); // prefix: '\\?\'
    let base_abs = PathAbs::new(base.as_ref()).unwrap();
    if let Some(rel_path) = pathdiff::diff_paths(path_abs, base_abs) {
        rel_path
    } else {
        panic!("fatal: path {:?} cannot convert to relative based on {:?}", path.as_ref(), base.as_ref());
    }
}

pub fn calc_file_hash(path: impl AsRef<Path>) -> io::Result<SHA1> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha1::new();

    let mut buffer = [0; 8192]; // 8K buffer
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let hash:[u8; 20] = hasher.finalize().into();
    Ok(SHA1(hash))
}

/// List all files in the given dir and its subdir, except `.libra`
pub fn list_files(path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_dir() {
        if path.file_name().unwrap_or_default() == ROOT_DIR {
            // ignore `.libra`
            return Ok(files);
        }
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // subdir
                files.extend(list_files(&path)?);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

/// list all files in the working dir(include subdir)
pub fn list_workdir_files() -> io::Result<Vec<PathBuf>> {
    list_files(&working_dir())
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

/// transform path to string, use '/' as separator even on windows
pub fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string() // TODO: test on windows
    // TODO maybe 'into_os_string().into_string().unwrap()' is good
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

    #[tokio::test]
    async fn test_to_workdir_path() {
        test::setup_with_new_libra().await;
        let workdir_path = to_workdir_path("src/main.rs");
        assert_eq!(workdir_path, PathBuf::from("src/main.rs"));
    }
}