use std::{
    error::Error,
    io::BufReader,
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::anyhow;
use common::config::BuildConfig;
use once_cell::sync::Lazy;
use serde_json::{Value, json};
use td_util::{command::spawn, file_io::file_writer};
use td_util_buck::{
    cells::CellInfo,
    run::{Buck2, targets_arguments},
    targets::Targets,
    types::{ProjectRelativePath, TargetLabel},
};
use tokio::{io::AsyncBufReadExt, process::Command, sync::mpsc::UnboundedSender, time::Duration};

// Import complete Error trait for better error handling
use crate::repo::changes::Changes;
use crate::{
    repo::{diff, sapling::status::Status},
    ws::{TaskPhase, WSMessage},
};

fn scorpio_base_url() -> String {
    crate::scorpio_api::base_url()
}

#[allow(dead_code)]
static PROJECT_ROOT: Lazy<String> =
    Lazy::new(|| std::env::var("BUCK_PROJECT_ROOT").expect("BUCK_PROJECT_ROOT must be set"));

const MOUNT_TIMEOUT_SECS: u64 = 7200;
static BUILD_CONFIG: Lazy<Option<BuildConfig>> = Lazy::new(load_build_config);

/// Mounts filesystem via remote API for repository access.
///
/// Initiates mount request and polls for completion with exponential backoff.
/// Required for accessing repository files during build process.
///
/// # Arguments
/// * `repo` - Repository path to mount
/// * `cl` - Change List identifier
///
/// # Returns
/// * `Ok(true)` - Mount operation completed successfully
/// * `Err(_)` - Mount request failed or timed out
#[allow(dead_code)]
#[deprecated(note = "This function is deprecated; use `mount_antares_fs` instead")]
pub async fn mount_fs(
    repo: &str,
    cl: Option<&str>,
    sender: Option<UnboundedSender<WSMessage>>,
    build_id: Option<String>,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
    // Mount operations may trigger remote repo fetching, dependency downloads,
    // or other network-heavy steps. To avoid premature timeouts when the network
    // is slow, we use a generous 2-hour timeout here. This can be tuned later.
    if let (Some(sender), Some(build_id)) = (&sender, &build_id)
        && let Err(err) = sender.send(WSMessage::TaskPhaseUpdate {
            id: build_id.clone(),
            phase: TaskPhase::DownloadingSource,
        })
    {
        tracing::error!("failed to send TaskPhaseUpdate (DownloadingSource): {err}");
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(MOUNT_TIMEOUT_SECS))
        .build()?;

    let mount_payload = if let Some(cl_id) = cl {
        json!({ "path": repo, "cl": cl_id })
    } else {
        json!({ "path": repo })
    };
    let mount_res = client
        .post(format!("{}/api/fs/mount", scorpio_base_url()))
        .header("Content-Type", "application/json")
        .body(mount_payload.to_string())
        .send()
        .await?;

    let mut mount_body: Value = mount_res.json().await?;

    if mount_body.get("status").and_then(|v| v.as_str()) != Some("Success") {
        let err_msg = mount_body
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Mount request failed");
        if err_msg == "please unmount" {
            // Unmount first
            #[allow(deprecated)]
            unmount_fs(repo, cl).await?;

            // Then retry the mount operation once
            let retry_mount_res = client
                .post(format!("{}/api/fs/mount", scorpio_base_url()))
                .header("Content-Type", "application/json")
                .body(mount_payload.to_string())
                .send()
                .await?;

            let retry_mount_body: Value = retry_mount_res.json().await?;

            // Check if retry was successful
            if retry_mount_body.get("status").and_then(|v| v.as_str()) != Some("Success") {
                let retry_err_msg = retry_mount_body
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Mount request failed after unmount");
                return Err(retry_err_msg.into());
            }

            // If retry was successful, continue with the new response
            mount_body = retry_mount_body;
        } else {
            return Err(err_msg.into());
        }
    }

    let request_id = mount_body
        .get("request_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing request_id in mount response")?
        .to_string();

    tracing::debug!("Mount request initiated with request_id: {}", request_id);

    // Check task status
    let select_url = format!("{}/api/fs/select/{request_id}", scorpio_base_url());
    let select_res = client.get(&select_url).send().await?;
    let select_body: Value = select_res.json().await?;

    tracing::debug!("Polling mount status : {:?}", select_body);

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
            Ok(true)
        }
        Some("error") => {
            let message = select_body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            Err(format!("Mount task failed: {message}").into())
        }
        #[allow(deprecated)]
        _ => unmount_fs(repo, cl).await,
    }
}

/// Mount an Antares File System (FS) repository.
///
/// # Arguments
/// - `repo`: The repository path to mount, e.g., `"my_repo"`.
/// - `cl`: Optional changelist ID. If provided, mounts the specified CL; otherwise, mounts the latest version.
///
/// # Returns
/// Returns a tuple `(mountpoint, mount_id)`:
/// - `mountpoint`: The path where the FS is mounted.
/// - `mount_id`: The unique ID of the mount operation.
///
/// # Errors
/// This function may return errors in the following cases:
/// - `reqwest::Error`: HTTP request failed or timed out.
/// - `serde_json::Error`: Failed to parse the response JSON.
/// - `Box<dyn Error + Send + Sync>`: Missing `mountpoint` or `mount_id` in the response.
///
/// # Example
/// ```no_run
/// use tokio;
/// use your_crate::mount_antares_fs;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let (mountpoint, mount_id) = mount_antares_fs("my_repo", Some("12345")).await?;
///     println!("Mounted at {} with id {}", mountpoint, mount_id);
///     Ok(())
/// }
/// ```
///
/// # Logging
/// - `debug`: Before sending request and payload details.
/// - `info`: Successfully mounted repository.
/// - `error`: HTTP request failure or missing fields in response.
pub async fn mount_antares_fs(
    job_id: &str,
    repo: &str,
    cl: Option<&str>,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    tracing::debug!(
        "Preparing to mount Antares FS for repo: {}, cl: {:?}",
        repo,
        cl
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(MOUNT_TIMEOUT_SECS))
        .build()
        .map_err(|e| {
            tracing::error!("Failed to build HTTP client: {:?}", e);
            e
        })?;

    let mount_payload = if let Some(cl_id) = cl {
        json!({ "path": repo, "cl": cl_id, "job_id": job_id })
    } else {
        json!({ "path": repo ,"job_id": job_id })
    };

    tracing::debug!("Sending mount request with payload: {}", mount_payload);

    let base = scorpio_base_url();
    let mount_res = client
        .post(format!("{base}/antares/mounts"))
        .header("Content-Type", "application/json")
        .body(mount_payload.to_string())
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to send mount request: {:?}", e);
            e
        })?;

    let mount_body: Value = mount_res.json().await.map_err(|e| {
        tracing::error!("Failed to parse mount response JSON: {:?}", e);
        e
    })?;

    let mountpoint = mount_body
        .get("mountpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            tracing::error!(
                "Missing 'mountpoint' in Antares mount response: {:?}",
                mount_body
            );
            "Missing mountpoint in Antares mount response"
        })?
        .to_string();

    let mount_id = mount_body
        .get("mount_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            tracing::error!(
                "Missing 'mount_id' in Antares mount response: {:?}",
                mount_body
            );
            "Missing mount_id in Antares mount response"
        })?
        .to_string();

    tracing::info!(
        "Antares mount created successfully: mountpoint={}, mount_id={}",
        mountpoint,
        mount_id
    );

    Ok((mountpoint, mount_id))
}

/// Asynchronously unmounts an Antares filesystem mount point.
///
/// This function sends an HTTP DELETE request to the local Antares service
/// to unmount the specified mount point.
///
/// # Arguments
/// - `mount_id`: The ID of the mount point to unmount. Usually assigned by the Antares service.
///
/// # Returns
/// - `Ok(true)`: Unmount succeeded and the state is `"Unmounted"`.
/// - `Err`: Unmount failed, which could be due to network errors, HTTP request failure,
///   or the mount state not being `"Unmounted"`.
///
/// # Error
/// - `Box<dyn Error + Send + Sync>`: Encapsulates various possible errors such as
///   request build failure, timeout, or JSON parsing failure.
///
/// # Example
/// ```no_run
/// use tracing_subscriber;
///
/// #[tokio::main]
/// async fn main() {
///     tracing_subscriber::fmt::init();
///     
///     match unmount_antares_fs("mount-1234").await {
///         Ok(true) => println!("Unmount succeeded"),
///         Err(e) => eprintln!("Unmount failed: {}", e),
///     }
/// }
/// ```
pub async fn unmount_antares_fs(mount_id: &str) -> Result<bool, Box<dyn Error + Send + Sync>> {
    tracing::info!("Starting unmount for mount_id: {}", mount_id);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(MOUNT_TIMEOUT_SECS))
        .build()?;

    let url = format!("{}/antares/mounts/{}", scorpio_base_url(), mount_id);
    tracing::debug!("Constructed DELETE URL: {}", url);

    let unmount_res = match client
        .delete(&url)
        .header("Content-Type", "application/json")
        .send()
        .await
    {
        Ok(res) => {
            tracing::info!(
                "HTTP DELETE request sent successfully for mount_id: {}",
                mount_id
            );
            res
        }
        Err(err) => {
            tracing::error!(
                "Failed to send HTTP DELETE request for mount_id {}: {}",
                mount_id,
                err
            );
            return Err(err.into());
        }
    };

    let unmount_body: Value = match unmount_res.json().await {
        Ok(json) => {
            tracing::debug!("Received response JSON: {}", json);
            json
        }
        Err(err) => {
            tracing::error!(
                "Failed to parse JSON response for mount_id {}: {}",
                mount_id,
                err
            );
            return Err(err.into());
        }
    };

    let unmount_state = unmount_body
        .get("state")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            let msg = "Missing 'state' in Antares mount response";
            tracing::error!("{}", msg);
            msg
        })?
        .to_string();

    if unmount_state == "Unmounted" {
        tracing::info!("Unmount succeeded for mount_id: {}", mount_id);
        Ok(true)
    } else {
        let err_msg = format!("Unmount failed, state: {}", unmount_state);
        tracing::error!("{}", err_msg);
        Err(err_msg.into())
    }
}

#[deprecated(note = "This function is deprecated; use `unmount_antares_fs` instead")]
async fn unmount_fs(repo: &str, cl: Option<&str>) -> Result<bool, Box<dyn Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let unmount_payload = if let Some(cl_id) = cl {
        serde_json::json!({
            "path": format!("{}_{}", repo.trim_start_matches('/'), cl_id)
        })
    } else {
        serde_json::json!({
            "path": format!("{}", repo.trim_start_matches('/'))
        })
    };

    let unmount_res = client
        .post(format!("{}/api/fs/unmount", scorpio_base_url()))
        .header("Content-Type", "application/json")
        .body(unmount_payload.to_string())
        .send()
        .await?;

    let unmount_body: serde_json::Value = unmount_res.json().await?;

    if unmount_body.get("status").and_then(|v| v.as_str()) != Some("Success") {
        let err_msg = unmount_body
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unmount request failed");
        return Err(err_msg.into());
    }

    Ok(true)
}

/// Buck2 targets stats every directory, which is slow on FUSE.
/// We pre-warm metadata with `ls -lR` to reduce statx latency.
/// TODO: Rewrite the targets logic in the monolith.
fn preheat(repo_path: &Path) -> anyhow::Result<()> {
    let preheat_status = std::process::Command::new("ls")
        .arg("-lR")
        .current_dir(repo_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if !preheat_status.success() {
        tracing::warn!("Preheat command finished with non-zero status, continuing anyway...");
    }
    Ok(())
}

/// Preheat a shallow directory tree to reduce cold-start metadata misses.
fn preheat_shallow(repo_path: &Path, max_depth: usize) -> anyhow::Result<()> {
    if max_depth == 0 {
        return Ok(());
    }

    let mut stack = vec![(repo_path.to_path_buf(), 0usize)];
    while let Some((path, depth)) = stack.pop() {
        let entries = match std::fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(err) => {
                tracing::warn!("Preheat shallow read_dir failed for {path:?}: {err}");
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    tracing::warn!("Preheat shallow entry error under {path:?}: {err}");
                    continue;
                }
            };

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    tracing::warn!(
                        "Preheat shallow file_type failed for {:?}: {err}",
                        entry.path()
                    );
                    continue;
                }
            };

            // Touch metadata to warm FUSE cache.
            let _ = entry.metadata();

            if file_type.is_dir() && depth < max_depth {
                stack.push((entry.path(), depth + 1));
            }
        }
    }

    Ok(())
}

fn preheat_shallow_depth() -> usize {
    if let Some(depth) = parse_env_usize("ORION_PREHEAT_SHALLOW_DEPTH") {
        return depth;
    }

    if let Some(depth) = parse_env_usize("MEGA_BUILD__ORION_PREHEAT_SHALLOW_DEPTH") {
        return depth;
    }

    build_config()
        .map(|config| config.orion_preheat_shallow_depth)
        .unwrap_or(0)
}

fn parse_env_usize(key: &str) -> Option<usize> {
    match std::env::var(key) {
        Ok(value) => match value.parse::<usize>() {
            Ok(depth) => Some(depth),
            Err(_) => {
                tracing::warn!("Invalid {key}={value:?}, ignoring.");
                None
            }
        },
        Err(_) => None,
    }
}

fn build_config() -> Option<&'static BuildConfig> {
    BUILD_CONFIG.as_ref()
}

fn load_build_config() -> Option<BuildConfig> {
    let path = resolve_config_path()?;
    let path_str = path.to_str()?;
    match common::config::Config::new(path_str) {
        Ok(config) => Some(config.build),
        Err(err) => {
            tracing::warn!("Failed to load MEGA config from {path:?}: {err}");
            None
        }
    }
}

fn resolve_config_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("MEGA_CONFIG") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
        tracing::warn!("MEGA_CONFIG points to missing file: {path:?}");
    }

    if let Ok(cwd) = std::env::current_dir() {
        let path = cwd.join("config/config.toml");
        if path.exists() {
            return Some(path);
        }
    }

    if let Ok(base_dir) = std::env::var("MEGA_BASE_DIR") {
        let path = PathBuf::from(base_dir).join("etc/config.toml");
        if path.exists() {
            return Some(path);
        }
    }

    None
}


/// Derive a stable per-mount buck2 isolation directory name.
///
/// Buck2 `--isolation-dir` expects a plain directory **name** (no path
/// separators).  Buck2 itself stores daemon state under
/// `<project_root>/.buck2/<isolation_dir>/`, so we only need to return a
/// unique name – not a full path.
fn buck2_isolation_dir(repo_path: &Path) -> anyhow::Result<String> {
    let digest = ring::digest::digest(
        &ring::digest::SHA256,
        repo_path.to_string_lossy().as_bytes(),
    );
    let suffix = &hex::encode(digest.as_ref())[..16];
    Ok(format!("buck2-isolation-{suffix}"))
}

/// Get target of a specific repo under tmp directory.
fn get_repo_targets(file_name: &str, repo_path: &Path) -> anyhow::Result<Targets> {
    const MAX_ATTEMPTS: usize = 2;
    let jsonl_path = PathBuf::from(repo_path).join(file_name);
    let isolation_dir = buck2_isolation_dir(repo_path)?;

    preheat(repo_path)?;

    for attempt in 1..=MAX_ATTEMPTS {
        tracing::debug!("Get targets for repo {repo_path:?} (attempt {attempt}/{MAX_ATTEMPTS})");
        let mut command = std::process::Command::new("buck2");
        command
            .args(targets_arguments())
            .args(["--isolation-dir", &isolation_dir]);
        command.current_dir(repo_path);
        let (mut child, stdout) = spawn(command)?;
        let mut writer = file_writer(&jsonl_path)?;
        std::io::copy(&mut BufReader::new(stdout), &mut writer)
            .map_err(|err| anyhow!("Failed to copy output to stdout: {}", err))?;
        writer
            .flush()
            .map_err(|err| anyhow!("Failed to flush writer: {}", err))?;
        let status = child.wait()?;
        if status.success() {
            return Targets::from_file(&jsonl_path);
        }

        tracing::warn!(
            "buck2 targets failed with status {} for repo {:?}",
            status,
            repo_path
        );
        if attempt < MAX_ATTEMPTS {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    Err(anyhow!(
        "buck2 targets failed after {MAX_ATTEMPTS} attempts for repo {:?}",
        repo_path
    ))
}

/// Run buck2-change-detector to get targets to build.
///
/// # Note
/// `mount_point` must be a mounted repository or CL path.
async fn get_build_targets(
    old_repo_mount_point: &str,
    mount_point: &str,
    mega_changes: Vec<Status<ProjectRelativePath>>,
) -> anyhow::Result<Vec<TargetLabel>> {
    tracing::info!("Get cells at {:?}", mount_point);
    let mount_path = PathBuf::from(mount_point);
    let old_repo = PathBuf::from(old_repo_mount_point);
    tracing::debug!("Analyzing changes {mega_changes:?}");

    preheat_shallow(&mount_path, preheat_shallow_depth())?;
    let mut buck2 = Buck2::with_root("buck2".to_string(), mount_path.clone());
    buck2.set_isolation_dir(buck2_isolation_dir(&mount_path)?);
    let mut cells = CellInfo::parse(
        &buck2
            .cells()
            .map_err(|err| anyhow!("Fail to get cells: {}", err))?,
    )?;

    tracing::debug!("Get config");
    cells.parse_config_data(
        &buck2
            .audit_config()
            .map_err(|err| anyhow!("Fail to get config: {}", err))?,
    )?;

    let base = get_repo_targets("base.jsonl", &old_repo)?;
    let changes = Changes::new(&cells, mega_changes)?;
    tracing::debug!("Changes {changes:?}");
    let diff = get_repo_targets("diff.jsonl", &mount_path)?;

    tracing::debug!("Base targets number: {}", base.len_targets_upperbound());
    tracing::debug!("Diff targets number: {}", diff.len_targets_upperbound());

    let immediate = diff::immediate_target_changes(&base, &diff, &changes, false);
    let recursive = diff::recursive_target_changes(&diff, &changes, &immediate, None, |_| true);

    Ok(recursive
        .into_iter()
        .flatten()
        .map(|(target, _)| target.label())
        .collect())
}

/// RAII guard for automatically unmounting Antares filesystem when dropped
struct MountGuard {
    mount_id: String,
    task_id: String,
    unmounted: AtomicBool,
}

impl MountGuard {
    fn new(mount_id: String, task_id: String) -> Self {
        Self {
            mount_id,
            task_id,
            unmounted: AtomicBool::new(false),
        }
    }

    async fn unmount(&self) {
        if self.unmounted.swap(true, Ordering::AcqRel) {
            return;
        }
        match unmount_antares_fs(&self.mount_id).await {
            Ok(_) => tracing::info!("[Task {}] Filesystem unmounted successfully.", self.task_id),
            Err(e) => {
                tracing::error!(
                    "[Task {}] Failed to unmount filesystem: {}",
                    self.task_id,
                    e
                )
            }
        }
    }
}

impl Drop for MountGuard {
    fn drop(&mut self) {
        if self.unmounted.load(Ordering::Acquire) {
            return;
        }
        let mount_id = self.mount_id.clone();
        let task_id: String = self.task_id.clone();
        // Spawn a task to handle the unmounting asynchronously
        // Since the unmount operation is idempotent, it's safe to perform asynchronously
        // even if the spawned task doesn't complete before program exit.
        // Multiple calls to unmount the same mount_id should be safe and have no side effects.
        tokio::spawn(async move {
            match unmount_antares_fs(&mount_id).await {
                Ok(_) => tracing::info!("[Task {}] Filesystem unmounted successfully.", task_id),
                Err(e) => tracing::error!("[Task {}] Failed to unmount filesystem: {}", task_id, e),
            }
        });
    }
}

/// Executes buck build with filesystem mounting and output streaming.
///
/// Process flow:
/// 1. Mount repository filesystem via remote API
/// 2. Execute buck build command with specified target and arguments  
/// 3. Stream build output in real-time via WebSocket
/// 4. Return final build status
///
/// # Arguments
/// * `id` - Build task identifier for logging and tracking
/// * `mount_path` - Monorepo path to mount (Buck2 project root or subdirectory)
/// * `target` - Buck build target specification  
/// * `args` - Additional command-line arguments for buck
/// * `cl` - Change List context identifier
/// * `sender` - WebSocket channel for streaming build output
/// * `changes` - Commit's file change information
///
/// # Returns
/// Process exit status indicating build success or failure
pub async fn build(
    id: String,
    repo: String,
    cl: String,
    sender: UnboundedSender<WSMessage>,
    changes: Vec<Status<ProjectRelativePath>>,
) -> Result<ExitStatus, Box<dyn Error + Send + Sync>> {
    tracing::info!("[Task {}] Building in repo {}", id, repo);

    // Handle empty cl string as None to mount the base repo without a CL layer.
    let cl_trimmed = cl.trim();
    let cl_arg = (!cl_trimmed.is_empty()).then_some(cl_trimmed);

    // Mount the entire monorepo root so all cells/toolchains are available,
    // but run buck2 from the specific sub-project directory.
    let mount_path = "/".to_string();
    // `repo_prefix` is the relative path from monorepo root to the buck2 project.
    // e.g., repo="/project/git-internal/git-internal" → repo_prefix="project/git-internal/git-internal"
    let repo_prefix = repo.strip_prefix('/').unwrap_or(&repo);

    // Changes are already relative to the sub-project (buck2 project root).
    // Do NOT prefix them with repo_prefix — buck2 runs from the sub-project dir.

    const MAX_TARGETS_ATTEMPTS: usize = 2;
    let mut mount_point = None;
    let mut mount_guard = None;
    let mut mount_guard_old_repo = None;
    let mut targets: Vec<TargetLabel> = Vec::new();
    let mut last_targets_error: Option<anyhow::Error> = None;

    for attempt in 1..=MAX_TARGETS_ATTEMPTS {
        // We should also mount the repo before cl, for build target analyzing.
        let id_for_old_repo = format!("{id}-old-{attempt}");
        let (old_repo_mount_point, mount_id_old_repo) =
            mount_antares_fs(&id_for_old_repo, &mount_path, None).await?;
        let guard_old_repo = MountGuard::new(mount_id_old_repo, id_for_old_repo);

        let id_for_repo = format!("{id}-{attempt}");
        let (repo_mount_point, mount_id) =
            mount_antares_fs(&id_for_repo, &mount_path, cl_arg).await?;
        let guard = MountGuard::new(mount_id.clone(), id_for_repo);

        tracing::info!(
            "[Task {}] Filesystem mounted successfully (attempt {}/{}).",
            id,
            attempt,
            MAX_TARGETS_ATTEMPTS
        );

        // Resolve the sub-project paths within each mount for buck2.
        let old_project_root = PathBuf::from(&old_repo_mount_point).join(repo_prefix);
        let new_project_root = PathBuf::from(&repo_mount_point).join(repo_prefix);

        match get_build_targets(
            old_project_root.to_str().unwrap_or(&old_repo_mount_point),
            new_project_root.to_str().unwrap_or(&repo_mount_point),
            changes.clone(),
        )
        .await
        {
            Ok(found_targets) => {
                mount_point = Some(repo_mount_point);
                mount_guard = Some(guard);
                mount_guard_old_repo = Some(guard_old_repo);
                targets = found_targets;
                break;
            }
            Err(e) => {
                guard.unmount().await;
                guard_old_repo.unmount().await;
                last_targets_error = Some(e);
                if attempt == MAX_TARGETS_ATTEMPTS {
                    break;
                }
                tracing::warn!(
                    "[Task {}] Failed to get build targets (attempt {}/{}): {}. Retrying with fresh mounts...",
                    id,
                    attempt,
                    MAX_TARGETS_ATTEMPTS,
                    last_targets_error.as_ref().unwrap()
                );
            }
        }
    }

    let mount_point = match mount_point {
        Some(value) => value,
        None => {
            let err = last_targets_error
                .map(|e| anyhow!("Error getting build targets: {e}"))
                .unwrap_or_else(|| anyhow!("Error getting build targets: mount point missing"));
            let error_msg = err.to_string();
            if sender
                .send(WSMessage::BuildOutput {
                    id: id.clone(),
                    output: error_msg.clone(),
                })
                .is_err()
            {
                tracing::error!(
                    "[Task {}] Failed to send BuildOutput for target discovery error",
                    id
                );
            }
            return Err(err.into());
        }
    };
    let mount_guard = mount_guard.ok_or("Mount guard missing after target discovery")?;
    let mount_guard_old_repo =
        mount_guard_old_repo.ok_or("Old repo mount guard missing after target discovery")?;

    let build_result = async {
        // Run buck2 build from the sub-project directory, not the monorepo root.
        // This ensures buck2 uses the sub-project's .buckconfig and PACKAGE files.
        let project_root = PathBuf::from(&mount_point).join(repo_prefix);
        let isolation_dir = buck2_isolation_dir(&project_root)?;
        let mut cmd = Command::new("buck2");
        cmd.args(["--isolation-dir", &isolation_dir]);
        let cmd = cmd
            .arg("build")
            .args(&targets)
            .arg("--target-platforms")
            .arg("prelude//platforms:default")
            // Avoid failing the whole build when a target is explicitly incompatible
            // with the selected platform (e.g., macOS-only crates on Linux builders).
            .arg("--skip-incompatible-targets")
            .arg("--verbose=2")
            .current_dir(&project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::debug!("[Task {}] Executing command: {:?}", id, cmd);

        let mut child = cmd.spawn()?;

        if let Err(e) = sender.send(WSMessage::TaskPhaseUpdate {
            id: id.clone(),
            phase: TaskPhase::RunningBuild,
        }) {
            tracing::error!("Failed to send RunningBuild phase update: {}", e);
        }

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let mut stdout_reader = tokio::io::BufReader::new(stdout).lines();
        let mut stderr_reader = tokio::io::BufReader::new(stderr).lines();

        let mut exit_status: Option<ExitStatus> = None;
        loop {
            tokio::select! {
                result = stdout_reader.next_line() => {
                    match result {
                        Ok(Some(line)) => {
                            if sender.send(WSMessage::BuildOutput { id: id.clone(), output: line }).is_err() {
                                child.kill().await?;
                                return Err("WebSocket connection lost during build.".into());
                            }
                        },
                        Ok(None) => break,
                        Err(e) => {
                            tracing::error!("[Task {}] Error reading stdout: {}", id, e);
                            break;
                        }
                    }
                },
                result = stderr_reader.next_line() => {
                    match result {
                        Ok(Some(line)) => {
                            if sender.send(WSMessage::BuildOutput { id: id.clone(), output: line }).is_err() {
                                child.kill().await?;
                                return Err("WebSocket connection lost during build.".into());
                            }
                        },
                        Ok(None) => break,
                        Err(e) => {
                            tracing::error!("[Task {}] Error reading stderr: {}", id, e);
                            break;
                        },
                    }
                },
                status = child.wait() => {
                    let status = status?;
                    tracing::info!("[Task {}] Buck2 process finished with status: {}", id, status);
                    exit_status = Some(status);
                    break;
                }
            }
        }

        if let Some(status) = exit_status {
            return Ok(status);
        }

        let status = child.wait().await?;
        Ok(status)
    }
    .await;

    mount_guard.unmount().await;
    mount_guard_old_repo.unmount().await;

    build_result
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use serde_json::json;
    use serial_test::serial;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_mount_antares_fs_success() {
        let listener = TcpListener::bind("127.0.0.1:2725").unwrap();
        let mock_server = MockServer::builder().listener(listener).start().await;

        Mock::given(method("POST"))
            .and(path("/antares/mounts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "mountpoint": "/mock/mountpoint",
                "mount_id": "mock_mount_id"
            })))
            .mount(&mock_server)
            .await;

        let result = mount_antares_fs("job1", &mock_server.uri(), None)
            .await
            .unwrap();
        assert_eq!(result.0, "/mock/mountpoint");
        assert_eq!(result.1, "mock_mount_id");
    }

    #[tokio::test]
    #[serial]
    async fn test_mount_antares_fs_missing_mountpoint() {
        let listener = TcpListener::bind("127.0.0.1:2725").unwrap();
        let mock_server = MockServer::builder().listener(listener).start().await;

        Mock::given(method("POST"))
            .and(path("/antares/mounts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "mount_id": "mock_mount_id"
            })))
            .mount(&mock_server)
            .await;

        let result = mount_antares_fs("job1", &mock_server.uri(), None).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing mountpoint in Antares mount response"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_mount_antares_fs_missing_mount_id() {
        let listener = TcpListener::bind("127.0.0.1:2725").unwrap();
        let mock_server = MockServer::builder().listener(listener).start().await;

        Mock::given(method("POST"))
            .and(path("/antares/mounts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "mountpoint": "/mock/mountpoint"
            })))
            .mount(&mock_server)
            .await;

        let result = mount_antares_fs("job1", &mock_server.uri(), None).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing mount_id in Antares mount response"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_unmount_antares_fs_success() {
        let listener = TcpListener::bind("127.0.0.1:2725").unwrap();
        let mock_server = MockServer::builder().listener(listener).start().await;

        Mock::given(method("DELETE"))
            .and(path("/antares/mounts/mock_mount_id"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "state": "Unmounted"
            })))
            .mount(&mock_server)
            .await;

        let result = unmount_antares_fs("mock_mount_id").await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    #[serial]
    async fn test_unmount_antares_fs_failure() {
        let listener = TcpListener::bind("127.0.0.1:2725").unwrap();
        let mock_server = MockServer::builder().listener(listener).start().await;

        Mock::given(method("DELETE"))
            .and(path("/antares/mounts/mock_mount_id"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "state": "Mounted"
            })))
            .mount(&mock_server)
            .await;

        let result = unmount_antares_fs("mock_mount_id").await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Unmount failed, state: Mounted"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_unmount_antares_fs_missing_state() {
        let listener = TcpListener::bind("127.0.0.1:2725").unwrap();
        let mock_server = MockServer::builder().listener(listener).start().await;

        Mock::given(method("DELETE"))
            .and(path("/antares/mounts/mock_mount_id"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&mock_server)
            .await;

        let result = unmount_antares_fs("mock_mount_id").await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing 'state' in Antares mount response"
        );
    }

    #[tokio::test]
    async fn test_mount_guard_creation() {
        let mount_guard = MountGuard::new("test_mount_id".to_string(), "test_task_id".to_string());
        assert_eq!(mount_guard.mount_id, "test_mount_id");
        assert_eq!(mount_guard.task_id, "test_task_id");
    }

    #[tokio::test]
    async fn test_mount_guard_drop_behavior() {
        let mount_guard = MountGuard::new("test_mount_id".to_string(), "test_task_id".to_string());
        assert_eq!(mount_guard.mount_id, "test_mount_id");
        assert_eq!(mount_guard.task_id, "test_task_id");

        let mount_id = mount_guard.mount_id.clone();
        tokio::spawn(async move {
            let _ = unmount_antares_fs(&mount_id).await;
        })
        .await
        .unwrap();
    }
}
