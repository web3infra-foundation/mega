use crate::repo::diff;
use crate::repo::sapling::status::Status;
use crate::ws::{TaskPhase, WSMessage};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::{Value, json};
use td_util_buck::types::{ProjectRelativePath, TargetLabel};
// Import complete Error trait for better error handling
use crate::repo::changes::Changes;
use anyhow::anyhow;
use std::error::Error;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use td_util::{command::spawn, file_io::file_writer};
use td_util_buck::{
    cells::CellInfo,
    run::{Buck2, targets_arguments},
    targets::Targets,
};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Duration;

#[allow(dead_code)]
static PROJECT_ROOT: Lazy<String> =
    Lazy::new(|| std::env::var("BUCK_PROJECT_ROOT").expect("BUCK_PROJECT_ROOT must be set"));

const MOUNT_TIMEOUT_SECS: u64 = 7200;

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
        .post("http://localhost:2725/api/fs/mount")
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
                .post("http://localhost:2725/api/fs/mount")
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
    let select_url = format!("http://localhost:2725/api/fs/select/{request_id}");
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

    let mount_res = client
        .post("http://localhost:2725/antares/mounts")
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

    let url = format!("http://localhost:2725/antares/mounts/{}", mount_id);
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
        .post("http://localhost:2725/api/fs/unmount")
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

/// Get target of a specific repo under tmp directory.
fn get_repo_targets(file_name: &str, repo_path: &Path) -> anyhow::Result<Targets> {
    tracing::debug!("Get targets for repo {repo_path:?}");
    let mut command = std::process::Command::new("buck2");
    command.args(targets_arguments());
    command.current_dir(repo_path);
    let (mut child, stdout) = spawn(command)?;
    let mut writer = file_writer(Path::new(file_name))?;
    std::io::copy(&mut BufReader::new(stdout), &mut writer)
        .map_err(|err| anyhow!("Failed to copy output to stdout: {}", err))?;
    writer
        .flush()
        .map_err(|err| anyhow!("Failed to flush writer: {}", err))?;
    child.wait()?;
    Targets::from_file(Path::new(file_name))
}

/// Run buck2-change-detector to get targets to build.
///
/// # Note
/// `mount_point` must be a mounted repository or CL path.
async fn get_build_targets(
    mount_point: &str,
    mega_changes: Vec<Status<ProjectRelativePath>>,
) -> anyhow::Result<Vec<TargetLabel>> {
    tracing::info!("Get cells at {:?}", mount_point);
    let mount_path = PathBuf::from(mount_point);
    let mut buck2 = Buck2::with_root("buck2".to_string(), mount_path.clone());
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

    let base = get_repo_targets("base.jsonl", &mount_path)?;
    let changes = Changes::new(&cells, mega_changes)?;
    let diff = get_repo_targets("diff.jsonl", &mount_path)?;

    tracing::debug!("Base targets number: {}", base.len_targets_upperbound());

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
}

impl MountGuard {
    fn new(mount_id: String, task_id: String) -> Self {
        Self { mount_id, task_id }
    }
}

impl Drop for MountGuard {
    fn drop(&mut self) {
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
/// * `repo` - Repository path for filesystem mounting
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
    tracing::info!("[Task {}] Building in repo '{}'", id, repo);

    let (mount_point, mount_id) = mount_antares_fs(&id, &repo, Some(&cl)).await?;
    let _mount_guard = MountGuard::new(mount_id.clone(), id.clone());
    let targets = get_build_targets(&mount_point, changes).await?;

    tracing::info!("[Task {}] Filesystem mounted successfully.", id);

    let mut cmd = Command::new("buck2");
    let cmd = cmd
        .arg("build")
        .args(&targets)
        .arg("--verbose=2")
        .current_dir(mount_point)
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
                let exit_status = status?;
                tracing::info!("[Task {}] Buck2 process finished with status: {}", id, exit_status);
                return Ok(exit_status);
            }
        }
    }

    let status = child.wait().await?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;
    use serde_json::json;
    use serial_test::serial;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
