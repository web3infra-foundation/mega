use once_cell::sync::Lazy;
use serde_json::{Value, json};
// Import complete Error trait for better error handling
use std::{error::Error, fs, path::{Path, PathBuf}, process::Command, time::{SystemTime, UNIX_EPOCH}};
use tokio::{time::{sleep, Duration}};

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

/// Mounts filesystem via remote API for repository access.
///
/// Initiates mount request and polls for completion with exponential backoff.
/// Required for accessing repository files during build process.
///
/// # Arguments
/// * `repo` - Repository path to mount
/// * `mr` - Merge request identifier
///
/// # Returns
/// * `Ok(true)` - Mount operation completed successfully
/// * `Err(_)` - Mount request failed or timed out
pub async fn mount_fs(repo: &str, mr: &str) -> Result<bool, Box<dyn Error + Send + Sync>> {
    tracing::debug!("Trying to mount {repo}, {mr}");
    let client = reqwest::Client::new();
    let mount_payload = json!({ "path": repo, "mr": mr });

    let mount_res = client
        .post("http://localhost:2725/api/fs/mount")
        .header("Content-Type", "application/json")
        .body(mount_payload.to_string())
        .send()
        .await?;

    let mount_body: Value = mount_res.json().await?;

    if mount_body.get("status").and_then(|v| v.as_str()) != Some("Success") {
        let err_msg = mount_body
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Mount request failed");
        return Err(err_msg.into());
    }

    let request_id = mount_body
        .get("request_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing request_id in mount response")?
        .to_string();

    tracing::debug!("Mount request initiated with request_id: {}", request_id);

    let max_attempts: u64 = std::env::var("SELECT_TASK_COUNT")
        .unwrap_or_else(|_| "30".to_string())
        .parse()
        .unwrap_or(30);

    let initial_poll_interval_secs: u64 = std::env::var("INITIAL_POLL_INTERVAL_SECS")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10);

    let max_poll_interval_secs = 120; // Maximum backoff interval: 2 minutes

    let mut poll_interval = initial_poll_interval_secs;

    for attempt in 1..=max_attempts {
        sleep(Duration::from_secs(poll_interval)).await;
        poll_interval = std::cmp::min(poll_interval * 2, max_poll_interval_secs);

        let select_url = format!("http://localhost:2725/api/fs/select/{request_id}");
        let select_res = client.get(&select_url).send().await?;
        let select_body: Value = select_res.json().await?;

        tracing::debug!(
            "Polling mount status (attempt {}/{}): {:?}",
            attempt,
            max_attempts,
            select_body
        );

        if select_body.get("status").and_then(|v| v.as_str()) != Some("Success") {
            let err_msg = select_body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Select request failed");
            return Err(err_msg.into());
        }

        match select_body.get("task_status").and_then(|v| v.as_str()) {
            Some("finished") => {
                tracing::info!(
                    "Mount task completed successfully for request_id: {}",
                    request_id
                );
                return Ok(true);
            }
            Some("error") => {
                let message = select_body
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                return Err(format!("Mount task failed: {message}").into());
            }
            _ => continue,
        }
    }

    Err("Mount operation timed out".into())
}

/// Unmount file system by the repo name which used on mount
pub async fn unmount_fs() -> Result<String, Box<dyn Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let repo = (*MR_DIR).clone();
    let mount_payload = json!({ "path": repo });

    let unmount_res = client
        .post("http://localhost:2725/api/fs/umount")
        .header("Content-Type", "application/json")
        .body(mount_payload.to_string())
        .send()
        .await?;
    let unmount_body: Value = unmount_res.json().await?;

    if unmount_body.get("status").and_then(|v| v.as_str()) != Some("Success") {
        let err_msg = unmount_body
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unmount request failed");
        return Err(err_msg.into());
    }

    Ok(unmount_body
        .get("message")
        .and_then(|v| Some(v.to_string()))
        .unwrap_or("Unmount successfully".to_string()))
}

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

/// Mount fs, copy and update a repo in tmp file from a mr in mr dir.
/// Typically, you should update the deletions and then call this function.
pub async fn merge_mr_tmp(repo: &str, mr: &str) -> bool {
    let res = mount_fs(&*MR_DIR, mr).await;
    if res.is_err() {
        return false;
    }
    let dest  = PathBuf::from(&*TMP_DIR);
    let src = PathBuf::from(&*PROJECT_ROOT).join("mount").join(&*MR_DIR).join(repo);
    copy_dir(src, dest, true)
}


/// Return the path to the dir that used to run commands.
pub fn repo_tmp_dir(repo: &str) -> PathBuf {
    PathBuf::from(&*TMP_DIR).join(repo)
}

/// Remove TMP_DIR
pub fn remove_tmp() -> bool {
    remove_objects(&[PathBuf::from(&*TMP_DIR)])
}