use crate::ws::WSMessage;
use once_cell::sync::Lazy;
use serde_json::{Value, json};
// Import complete Error trait for better error handling
use std::error::Error;
use std::process::{ExitStatus, Stdio};
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
pub async fn mount_fs(repo: &str, cl: &str) -> Result<bool, Box<dyn Error + Send + Sync>> {
    // Mount operations may trigger remote repo fetching, dependency downloads,
    // or other network-heavy steps. To avoid premature timeouts when the network
    // is slow, we use a generous 2-hour timeout here. This can be tuned later.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(MOUNT_TIMEOUT_SECS))
        .build()?;

    let mount_payload = json!({ "path": repo, "cl": cl });
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

async fn unmount_fs(repo: &str, cl: &str) -> Result<bool, Box<dyn Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let unmount_payload = serde_json::json!({
        "path": format!("{}_{}", repo.trim_start_matches('/'), cl)
    });

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
///
/// # Returns
/// Process exit status indicating build success or failure
pub async fn build(
    id: String,
    repo: String,
    target: String,
    _args: Vec<String>,
    cl: String,
    sender: UnboundedSender<WSMessage>,
) -> Result<ExitStatus, Box<dyn Error + Send + Sync>> {
    tracing::info!(
        "[Task {}] Building target '{}' in repo '{}'",
        id,
        target,
        repo
    );

    mount_fs(&repo, &cl).await?;
    tracing::info!("[Task {}] Filesystem mounted successfully.", id);

    let mut cmd = Command::new("buck2");
    let cmd = cmd
        .arg("build")
        .arg(&target)
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
    match unmount_fs(&repo, &cl).await {
        Ok(_) => tracing::info!("[Task {}] Filesystem unmounted successfully.", id),
        Err(e) => tracing::error!("[Task {}] Failed to unmount filesystem: {}", id, e),
    }
    Ok(status)
}
