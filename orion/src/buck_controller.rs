use crate::ws::WSMessage;
use once_cell::sync::Lazy;
use serde_json::{Value, json};
// Import complete Error trait for better error handling
use std::error::Error;
use std::process::{ExitStatus, Stdio};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::{Duration, sleep};

static PROJECT_ROOT: Lazy<String> =
    Lazy::new(|| std::env::var("BUCK_PROJECT_ROOT").expect("BUCK_PROJECT_ROOT must be set"));

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
/// * `mr` - Merge request context identifier
/// * `sender` - WebSocket channel for streaming build output
///
/// # Returns
/// Process exit status indicating build success or failure
pub async fn build(
    id: String,
    repo: String,
    target: String,
    args: Vec<String>,
    mr: String,
    sender: UnboundedSender<WSMessage>,
) -> Result<ExitStatus, Box<dyn Error + Send + Sync>> {
    tracing::info!(
        "[Task {}] Building target '{}' in repo '{}'",
        id,
        target,
        repo
    );

    mount_fs(&repo, &mr).await?;
    tracing::info!("[Task {}] Filesystem mounted successfully.", id);

    let mut cmd = Command::new("buck2");
    let cmd = cmd
        .arg("build")
        .args(args)
        .arg(&target)
        .current_dir(format!("{}/{}", *PROJECT_ROOT, repo))
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
    Ok(status)
}
