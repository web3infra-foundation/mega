use once_cell::sync::Lazy;
use std::{fs, path::{Path, PathBuf}, process::Command, time::{SystemTime, UNIX_EPOCH}};

/// The directory this module use to store data, mount repo, build target.
static PROJECT_ROOT: Lazy<String> =
    Lazy::new(|| std::env::var("BUCK_PROJECT_ROOT").expect("BUCK_PROJECT_ROOT must be set"));
/// The under relevant to PROJECT_ROOT/mount, used to download mr files.
static MR_DIR: Lazy<String> = Lazy::new(|| std::env::var("MR_REPO").expect("MR_REPO must be set"));
static TMP_DIR: Lazy<String> = Lazy::new(|| {
    let mut path = PathBuf::from(&std::env::var("BUILD_TMP").expect("BUILD_TMP must be set"));
    let stamp = {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("invalid time")
            .as_nanos() as u64;
        format!("{:06x}", seed % 1000000) 
    };
    path.push(&stamp);
    path.to_string_lossy().to_string()
});

/// Remove the files or dirs, return whether the operation is successful.
pub fn remove_objects(files: &[PathBuf]) -> bool {
    tracing::debug!("Deleting {files:?}");
    for path in files {
        if path.is_file() {
            let res = fs::remove_file(path);
            if let Err(e) = res {
                tracing::error!("Can't remove file {path:?}: {e}");
                return false;
            }
        } else {
            let res = fs::remove_dir_all(path);
            if let Err(e) = res {
                tracing::error!("Can't remove dir {path:?}: {e}");
                return false;
            }
        }
    }
    true
}

/// Copy the directory from src to dest. If `update` is set, use `cp -ur.
/// Return whether the operation is successful
/// 
/// This funtion is blocking, it seems no nessesity to use async.
pub fn copy_dir<P: AsRef<Path>>(src: P, dest: P, update: bool) -> bool {
    let src = src.as_ref();
    let dest = dest.as_ref();
    tracing::debug!("Copying {src:?} to {dest:?}, update: {update}");
    let argu = if update {
        "-ur"
    } else {
        "-r"
    };
    let cp = Command::new("cp")
        .arg(argu)
        .arg(src)
        .arg(dest)
        .output();
    match cp {
        Ok(output) => {
            tracing::debug!("Copy {src:?} to {dest:?}: {:?}", String::from_utf8(output.stderr));
            true
        }
        Err(err) => {
            tracing::error!("Failed to copy dir {src:?} to {dest:?}: {err}");
            false
        }
    }
}

/// Copy a whole repo from scorpio mounted dir to tmp dir.
/// Assume tmp dir exists.
/// Only copy original files, not include mr.
/// 
/// Return whether the operation success.
pub fn copy_repo(repo: &str) -> bool {
    let src = PathBuf::from(&*PROJECT_ROOT).join("mount").join(repo);
    let dest = PathBuf::from(&*TMP_DIR);
    if !dest.exists() {
        fs::create_dir(&dest).unwrap();
    }
    copy_dir(src, dest, false)
}


/// Return the path to the dir that used to run commands.
pub fn repo_tmp_dir(repo: &str) -> PathBuf {
    PathBuf::from(&*TMP_DIR).join(repo)
}

/// Remove TMP_DIR
pub fn remove_tmp() -> bool {
    remove_objects(&[PathBuf::from(&*TMP_DIR)])
}
