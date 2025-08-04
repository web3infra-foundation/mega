use crate::ws::WSMessage;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::io;
use std::process::{ExitStatus, Stdio};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::{sleep, Duration};

static PROJECT_ROOT: Lazy<String> =
    Lazy::new(|| std::env::var("BUCK_PROJECT_ROOT").expect("BUCK_PROJECT_ROOT must be set"));

/// Sends a filesystem mount request to the specified API endpoint and waits for completion
/// Parameters:
/// - repo: Repository path to mount
/// - mr: Merge request number
///   Returns: Result containing success status on completion, or an error on failure
pub async fn mount_fs(repo: &str, mr: &str) -> Result<bool, Box<dyn std::error::Error>> {
    // Create HTTP client
    let client = reqwest::Client::new();

    // Step 1: Send mount request to get request_id
    let mount_payload = json!({
        "path": repo,
        "mr": mr,
    });

    let mount_res = client
        .post("http://localhost:2725/api/fs/mount")
        .header("Content-Type", "application/json")
        .body(mount_payload.to_string())
        .send()
        .await?;

    println!("Mount request status: {}", mount_res.status());

    let mount_body: Value = mount_res.json().await?;
    println!("Mount response: {mount_body}");

    // Extract request_id from response
    let request_id = mount_body
        .get("request_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing request_id in mount response")?
        .to_string();

    // Check if mount request was successful
    if mount_body.get("status").and_then(|v| v.as_str()) != Some("Success") {
        return Err("Mount request failed".into());
    }

    println!("Mount request initiated with request_id: {request_id}");

    // Step 2: Poll for completion
    let max_attempts = std::env::var("SELECT_TASK_COUNT").unwrap_or("30".into());
    let max_attempts: u64 = max_attempts.parse().unwrap_or(30);
    let initial_poll_interval_secs =
        std::env::var("INITIAL_POLL_INTERVAL_SECS").unwrap_or("10".into());
    let max_poll_interval_secs = 120; // Maximum backoff interval: 2 minutes
    let mut poll_interval = initial_poll_interval_secs.parse::<u64>().unwrap_or(10);
    for _attempt in 1..=max_attempts {
        // Wait before checking status
        sleep(Duration::from_secs(poll_interval)).await;
        // Exponential backoff: double interval, up to max_poll_interval_secs
        poll_interval = std::cmp::min(poll_interval * 2, max_poll_interval_secs);

        let select_url = format!("http://localhost:2725/api/fs/select/{request_id}");
        let select_res = client.get(&select_url).send().await?;

        let select_body: Value = select_res.json().await?;
        println!("Select response: {select_body}");

        // Check overall status
        if select_body.get("status").and_then(|v| v.as_str()) != Some("Success") {
            return Err("Select request failed".into());
        }

        // Check task status
        match select_body.get("task_status").and_then(|v| v.as_str()) {
            Some("finished") => {
                println!("Mount task completed successfully");
                return Ok(true);
            }
            Some("error") => {
                let message = select_body
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                return Err(format!("Mount task failed: {message}").into());
            }
            Some("fetching") => {
                println!("Mount task still in progress (fetching)...");
                continue;
            }
            Some(other_status) => {
                println!("Mount task status: {other_status}");
                continue;
            }
            None => {
                return Err("Missing task_status in select response".into());
            }
        }
    }
    Err("Mount operation timed out".into())
}

pub async fn build(
    id: String,
    repo: String,
    target: String,
    args: Vec<String>,
    mr: String,
    sender: UnboundedSender<WSMessage>,
) -> io::Result<ExitStatus> {
    tracing::info!("Building {} in repo {} with target {}", id, repo, target);
    // Prepare the command to run
    // Note: `args` is a list of additional arguments to pass to the `buck

    // Mount filesystem before building
    match mount_fs(&repo, &mr).await {
        Ok(true) => {
            tracing::info!("Filesystem mounted successfully for repo: {}", repo);
        }
        Ok(false) => {
            tracing::error!("Filesystem mount failed for repo: {}", repo);
            return Err(io::Error::other("Filesystem mount failed"));
        }
        Err(e) => {
            tracing::error!("Error mounting filesystem for repo {}: {}", repo, e);
            return Err(io::Error::other(format!("Filesystem mount error: {e}")));
        }
    }

    let mut cmd = Command::new("buck2");
    let cmd = cmd
        .arg("build")
        .args(args)
        .arg(target)
        .current_dir(format!("{}/{}", *PROJECT_ROOT, repo))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // actually, some info (like: "BUILD SUCCESSFUL") is printed to stderr

    tracing::debug!("cmd:{:?}", cmd);

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
                        sender.send(WSMessage::BuildOutput {
                            id: id.clone(),
                            output: line.clone(),
                        }).unwrap();
                    },
                    Err(_) => break,
                    _ => (),
                }
            }
            result = stderr_reader.next_line() => {
                match result {
                    Ok(Some(line)) => {
                        sender.send(WSMessage::BuildOutput {
                            id: id.clone(),
                            output: line.clone(),
                        }).unwrap();
                    },
                    Err(_) => break,
                    _ => (),
                }
            }
            result = child.wait() => {
                return result;
            }
        }
    }

    let status = child.wait().await?;
    Ok(status)
}
