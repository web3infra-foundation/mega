use indicatif::{ProgressBar, ProgressStyle};
use mercury::hash::SHA1;
use mercury::internal::object::types::ObjectType;
use path_absolutize::*;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use crate::utils::client_storage::ClientStorage;
use crate::utils::path;
use crate::utils::path_ext::PathExt;

use ignore::{gitignore::Gitignore, Match};

pub const ROOT_DIR: &str = ".libra";
pub const DATABASE: &str = "libra.db";
pub const ATTRIBUTES: &str = ".libra_attributes";

/// Returns the current working directory as a `PathBuf`.
///
/// This function wraps the `std::env::current_dir()` function and unwraps the result.
/// If the current directory value is not available for any reason, this function will panic.
///
/// TODO - Add additional check result from `std::env::current_dir()` to handle the panic
///
/// # Returns
///
/// A `PathBuf` representing the current working directory.
pub fn cur_dir() -> PathBuf {
    env::current_dir().unwrap()
}

/// Try to get the storage path of the repository, which is the path of the `.libra` directory
/// - if the current directory or given path is not a repository, return an error
pub fn try_get_storage_path(path: Option<PathBuf>) -> Result<PathBuf, io::Error> {
    let mut path = path.clone().unwrap_or_else(cur_dir);
    let orig = path.clone();
    loop {
        let libra = path.join(ROOT_DIR);
        if libra.exists() {
            return Ok(libra);
        }
        if !path.pop() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{:?} is not a libra repository", orig),
            ));
        }
    }
}

/// Load the storage path with optional given repository
pub fn storage_path() -> PathBuf {
    try_get_storage_path(None).unwrap()
}

/// Check if libra repo exists
pub fn check_repo_exist() -> bool {
    if try_get_storage_path(None).is_err() {
        eprintln!("fatal: not a libra repository (or any of the parent directories): .libra");
        return false;
    }
    true
}

/// Get `ClientStorage` for the `objects` directory
pub fn objects_storage() -> ClientStorage {
    ClientStorage::init(path::objects())
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

/// Judge if the path is a sub path of the parent path
/// - Not check existence
/// - `true` if path == parent
pub fn is_sub_path<P, B>(path: P, parent: B) -> bool
where
    P: AsRef<Path>,
    B: AsRef<Path>,
{
    // to absolute, just for clear redundant `..` `.` in the path
    // may generate wrong intermediate path, but the final result is correct (after `starts_with`)
    let path_abs = path.as_ref().absolutize().unwrap();
    let parent_abs = parent.as_ref().absolutize().unwrap();
    path_abs.starts_with(parent_abs)
}

/// Judge if the `path` is sub-path of `paths`(include sub-dirs)
/// - absolute path or relative path to the current dir
/// - Not check existence
pub fn is_sub_of_paths<P, U>(path: impl AsRef<Path>, paths: U) -> bool
where
    P: AsRef<Path>,
    U: IntoIterator<Item = P>,
{
    for p in paths {
        if is_sub_path(path.as_ref(), p.as_ref()) {
            return true;
        }
    }
    false
}

/// Filter paths to fit the given paths, include sub-dirs
/// - return the paths that are sub-path of the fit paths
/// - `paths`: to workdir
/// - `fit_paths`: abs or rel
/// - Not check existence
pub fn filter_to_fit_paths<P>(paths: &[P], fit_paths: &Vec<P>) -> Vec<P>
where
    P: AsRef<Path> + Clone,
{
    paths
        .iter()
        .filter(|p| {
            let p = workdir_to_absolute(p.as_ref());
            is_sub_of_paths(p, fit_paths)
        })
        .cloned()
        .collect()
}

/// `path` & `base` must be absolute or relative (to current dir)
/// <br> return "." if `path` == `base`
pub fn to_relative<P, B>(path: P, base: B) -> PathBuf
where
    P: AsRef<Path>,
    B: AsRef<Path>,
{
    // × crate `PathAbs` is NOT good enough
    // 1. `PathAbs::new` can not handle `.` or `./`, all return ""
    // 2. `PathAbs::new` generate prefix: '\\?\' on Windows
    // So, we replace it with `path_absolutize` √
    let path_abs = path.as_ref().absolutize().unwrap();
    let base_abs = base.as_ref().absolutize().unwrap();
    if let Some(rel_path) = pathdiff::diff_paths(path_abs, base_abs) {
        if rel_path.to_string_lossy() == "" {
            PathBuf::from(".")
        } else {
            rel_path
        }
    } else {
        panic!(
            "fatal: path {:?} cannot convert to relative based on {:?}",
            path.as_ref(),
            base.as_ref()
        );
    }
}

#[allow(dead_code)]
/// Convert a path to relative path to the current directory
/// - `path` must be absolute or relative (to current dir)
pub fn to_current_dir<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    to_relative(path, cur_dir())
}

/// Convert a workdir path to relative path
/// - `base` must be absolute or relative (to current dir)
pub fn workdir_to_relative<P, B>(path: P, base: B) -> PathBuf
where
    P: AsRef<Path>,
    B: AsRef<Path>,
{
    let path_abs = workdir_to_absolute(path);
    to_relative(path_abs, base)
}

/// Convert a workdir path to relative path to the current directory
pub fn workdir_to_current<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    workdir_to_relative(path, cur_dir())
}

/// List all files in the given dir and its sub_dir, except `.libra`
/// - input `path`: absolute path or relative path to the current dir
/// - output: to workdir path
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
                files.extend(list_files(&path)?);
            } else {
                files.push(to_workdir_path(&path));
            }
        }
    }
    Ok(files)
}

/// list all files in the working dir(include sub_dir)
/// - output: to workdir path
pub fn list_workdir_files() -> io::Result<Vec<PathBuf>> {
    list_files(&working_dir())
}

/// Integrate the input paths (relative, absolute, file, dir) to workdir paths
/// - only include existing files
pub fn integrate_pathspec(paths: &Vec<PathBuf>) -> HashSet<PathBuf> {
    let mut workdir_paths = HashSet::new();
    for path in paths {
        if path.is_dir() {
            let files = list_files(path).unwrap(); // to workdir
            workdir_paths.extend(files);
        } else {
            workdir_paths.insert(path.to_workdir());
        }
    }
    workdir_paths
}

/// write content to file
/// - create parent directory if not exist
pub fn write_file(content: &[u8], file: &PathBuf) -> io::Result<()> {
    let mut parent = file.clone();
    parent.pop();
    fs::create_dir_all(parent)?;
    let mut file = fs::File::create(file)?;
    file.write_all(content)
}

/// Removing the empty directories in cascade until meet the root of workdir or the current dir
pub fn clear_empty_dir(dir: &Path) {
    let mut dir = if dir.is_dir() {
        dir.to_path_buf()
    } else {
        dir.parent().unwrap().to_path_buf()
    };

    let repo = storage_path();
    // CAN NOT remove .libra & current dir
    while !is_sub_path(&repo, &dir) && !is_cur_dir(&dir) {
        if is_empty_dir(&dir) {
            fs::remove_dir(&dir).unwrap();
        } else {
            break; // once meet a non-empty dir, stop
        }
        dir.pop();
    }
}

pub fn is_empty_dir(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    fs::read_dir(dir).unwrap().next().is_none()
}

pub fn is_cur_dir(dir: &Path) -> bool {
    dir.absolutize().unwrap() == cur_dir().absolutize().unwrap()
}

/// transform path to string, use '/' as separator even on windows
/// TODO test on windows
/// TODO maybe 'into_os_string().into_string().unwrap()' is good
pub fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

/// extend hash, panic if not valid or ambiguous
pub async fn get_commit_base(commit_base: &str) -> Result<SHA1, String> {
    let storage = objects_storage();

    let commits = storage.search(commit_base).await;
    if commits.is_empty() {
        return Err(format!("fatal: invalid reference: {}", commit_base));
    } else if commits.len() > 1 {
        return Err(format!("fatal: ambiguous argument: {}", commit_base));
    }
    if !storage.is_object_type(&commits[0], ObjectType::Commit) {
        Err(format!(
            "fatal: reference is not a commit: {}, is {}",
            commit_base,
            storage.get_object_type(&commits[0]).unwrap()
        ))
    } else {
        Ok(commits[0])
    }
}

/// Get the repository name from the url
/// - e.g. `https://github.com/web3infra-foundation/mega.git/` -> mega
/// - e.g. `https://github.com/web3infra-foundation/mega.git` -> mega
pub fn get_repo_name_from_url(mut url: &str) -> Option<&str> {
    if url.ends_with('/') {
        url = &url[..url.len() - 1];
    }
    let repo_start = url.rfind('/')? + 1;
    let repo_end = url.rfind('.')?;
    Some(&url[repo_start..repo_end])
}

/// Find the appropriate unit and value for Bytes.
/// ### Examples
/// - 1024 bytes -> 1 KiB
/// - 1024 * 1024 bytes -> 1 MiB
pub fn auto_unit_bytes(bytes: u64) -> byte_unit::AdjustedByte {
    let bytes = byte_unit::Byte::from(bytes);
    bytes.get_appropriate_unit(byte_unit::UnitType::Binary)
}
/// Create a default style progress bar
pub fn default_progress_bar(len: u64) -> ProgressBar {
    let progress_bar = ProgressBar::new(len);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.magenta} [{elapsed_precise}] [{bar:40.green/white}] {bytes}/{total_bytes} ({eta}) {bytes_per_sec}")
            .unwrap()
            .progress_chars("=> "),
    );
    progress_bar
}

/// Check each directory level from `work_dir` to `target_file` to see if there is a `.gitignore` file that matches `target_file`.
///
/// Assume `target_file` is `in work_dir`.
pub fn check_gitignore(work_dir: &PathBuf, target_file: &PathBuf) -> bool {
    assert!(target_file.starts_with(work_dir));
    let mut dir = target_file.clone();
    dir.pop();

    while dir.starts_with(work_dir) {
        let mut cur_file = dir.clone();
        cur_file.push(".libraignore");
        if cur_file.exists() {
            let (ignore, err) = Gitignore::new(&cur_file);
            if let Some(e) = err {
                println!(
                    "warning: There are some invalid globs in libraignore file {:#?}:\n{}\n",
                    cur_file, e
                );
            }
            if let Match::Ignore(_) = ignore.matched(target_file, false) {
                return true;
            }
        }
        dir.pop();
    }

    false
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::test;
    use serial_test::serial;
    use std::env;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    ///Test get current directory success.
    fn cur_dir_returns_current_directory() {
        let expected = env::current_dir().unwrap();
        let actual = cur_dir();
        assert_eq!(actual, expected);
    }

    #[test]
    #[serial]
    #[ignore]
    ///Test the function of is_sub_path.
    fn test_is_sub_path() {
        let _guard = test::ChangeDirGuard::new(Path::new(env!("CARGO_MANIFEST_DIR")));

        assert!(is_sub_path("src/main.rs", "src"));
        assert!(is_sub_path("src/main.rs", "src/"));
        assert!(is_sub_path("src/main.rs", "src/main.rs"));
        assert!(is_sub_path("src/main.rs", "."));
    }

    #[test]
    ///Test the function of to_relative.
    fn test_to_relative() {
        assert_eq!(to_relative("src/main.rs", "src"), PathBuf::from("main.rs"));
        assert_eq!(to_relative(".", "src"), PathBuf::from(".."));
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    ///Test the function of to_workdir_path.
    async fn test_to_workdir_path() {
        let temp_path = tempdir().unwrap();
        test::setup_with_new_libra_in(temp_path.path()).await;
        let _guard = test::ChangeDirGuard::new(temp_path.path());

        assert_eq!(
            to_workdir_path("./src/abc/../main.rs"),
            PathBuf::from("src/main.rs")
        );
        assert_eq!(to_workdir_path("."), PathBuf::from("."));
        assert_eq!(to_workdir_path("./"), PathBuf::from("."));
        assert_eq!(to_workdir_path(""), PathBuf::from("."));
    }

    #[test]
    #[serial]
    #[ignore]
    /// Tests that files matching patterns in .libraignore are correctly identified as ignored.
    fn test_check_gitignore_ignore() {
        let temp_path = tempdir().unwrap();
        let _guard = test::ChangeDirGuard::new(temp_path.path());

        let mut gitignore_file = fs::File::create(".libraignore").unwrap();
        gitignore_file.write_all(b"*.bar").unwrap();

        let target = temp_path.path().join("tmp/foo.bar");
        assert!(check_gitignore(&temp_path.keep(), &target));
    }

    #[test]
    #[serial]
    #[ignore]
    /// Tests ignore pattern matching in subdirectories with .libraignore files at different directory levels.
    fn test_check_gitignore_ignore_subdirectory() {
        let temp_path = tempdir().unwrap();
        let _guard = test::ChangeDirGuard::new(temp_path.path());

        fs::create_dir_all("tmp").unwrap();
        fs::create_dir_all("tmp/tmp1").unwrap();
        fs::create_dir_all("tmp/tmp1/tmp2").unwrap();
        let mut gitignore_file1 = fs::File::create("tmp/.libraignore").unwrap();
        gitignore_file1.write_all(b"*.bar").unwrap();
        let workdir = env::current_dir().unwrap();
        let target = workdir.join("tmp/tmp1/tmp2/foo.bar");
        assert!(check_gitignore(&workdir, &target));
        fs::remove_dir_all(workdir.join("tmp")).unwrap();
    }

    #[test]
    #[serial]
    #[ignore]
    /// Tests that files not matching patterns in .libraignore are correctly identified as not ignored.
    fn test_check_gitignore_not_ignore() {
        let temp_path = tempdir().unwrap();
        let _guard = test::ChangeDirGuard::new(temp_path.path());

        let mut gitignore_file = fs::File::create(".libraignore").unwrap();
        gitignore_file.write_all(b"*.bar").unwrap();
        let workdir = env::current_dir().unwrap();
        let target = workdir.join("tmp/bar.foo");
        assert!(!check_gitignore(&workdir, &target));
        fs::remove_file(workdir.join(".libraignore")).unwrap();
    }

    #[test]
    #[serial]
    #[ignore]
    /// Tests that files not matching subdirectory-specific patterns in .libraignore are correctly identified as not ignored.
    fn test_check_gitignore_not_ignore_subdirectory() {
        let temp_path = tempdir().unwrap();
        let _guard = test::ChangeDirGuard::new(temp_path.path());

        fs::create_dir_all("tmp").unwrap();
        fs::create_dir_all("tmp/tmp1").unwrap();
        fs::create_dir_all("tmp/tmp1/tmp2").unwrap();
        let mut gitignore_file1 = fs::File::create("tmp/.libraignore").unwrap();
        gitignore_file1.write_all(b"tmp/tmp1/tmp2/*.bar").unwrap();
        let workdir = env::current_dir().unwrap();
        let target = workdir.join("tmp/tmp1/tmp2/foo.bar");
        assert!(!check_gitignore(&workdir, &target));
        fs::remove_dir_all(workdir.join("tmp")).unwrap();
    }
}
