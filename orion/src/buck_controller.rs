use crate::repo::diff;
use crate::repo::sapling::status::Status;
use crate::ws::WSMessage;
use once_cell::sync::Lazy;
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
pub async fn mount_fs(repo: &str, cl: Option<&str>) -> Result<bool, Box<dyn Error + Send + Sync>> {
    // Mount operations may trigger remote repo fetching, dependency downloads,
    // or other network-heavy steps. To avoid premature timeouts when the network
    // is slow, we use a generous 2-hour timeout here. This can be tuned later.
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
        _ => unmount_fs(repo, cl).await,
    }
}

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
/// `{repo}` and `{repo}_{cl}` directories must be mount before invoking.
async fn get_build_targets(
    repo: &str,
    cl: &str,
    mega_changes: Vec<Status<ProjectRelativePath>>,
) -> anyhow::Result<Vec<TargetLabel>> {
    let repo_path = PathBuf::from(&format!("{}{}", *PROJECT_ROOT, repo));
    let repo_cl_path = PathBuf::from(&format!("{}{}_{}", *PROJECT_ROOT, repo, cl));
    tracing::info!("Get cells at {:?}", repo_path);
    let mut buck2 = Buck2::with_root("buck2".to_string(), repo_cl_path.clone());
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

    let base = get_repo_targets("base.jsonl", &repo_path)?;
    let changes = Changes::new(&cells, mega_changes)?;
    let diff = get_repo_targets("diff.jsonl", &repo_cl_path)?;

    tracing::debug!("Base targets number: {}", base.len_targets_upperbound());

    let immediate = diff::immediate_target_changes(&base, &diff, &changes, false);
    let recursive = diff::recursive_target_changes(&diff, &changes, &immediate, None, |_| true);

    Ok(recursive
        .into_iter()
        .flatten()
        .map(|(target, _)| target.label())
        .collect())
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
    _args: Vec<String>,
    cl: String,
    sender: UnboundedSender<WSMessage>,
    changes: Vec<Status<ProjectRelativePath>>,
) -> Result<ExitStatus, Box<dyn Error + Send + Sync>> {
    tracing::info!("[Task {}] Building in repo '{}'", id, repo);

    mount_fs(&repo, Some(&cl)).await?;
    mount_fs(&repo, None).await?;
    let targets = get_build_targets(&repo, &cl, changes).await?;

    tracing::info!("[Task {}] Filesystem mounted successfully.", id);

    let mut cmd = Command::new("buck2");
    let cmd = cmd
        .arg("build")
        .args(&targets)
        .current_dir(format!("{}{}_{}", *PROJECT_ROOT, repo, cl))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    tracing::debug!("[Task {}] Executing command: {:?}", id, cmd);

    let mut child = cmd.spawn()?;
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

    // Clean up the mount dir
    match unmount_fs(&repo, Some(&cl)).await {
        Ok(_) => tracing::info!("[Task {}] Filesystem unmounted successfully.", id),
        Err(e) => tracing::error!("[Task {}] Failed to unmount filesystem: {}", id, e),
    }
    match unmount_fs(&repo, None).await {
        Ok(_) => tracing::info!("[Task {}] Filesystem unmounted successfully.", id),
        Err(e) => tracing::error!("[Task {}] Failed to unmount filesystem: {}", id, e),
    }
    Ok(status)
}
