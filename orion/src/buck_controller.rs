use crate::ws::WSMessage;
use once_cell::sync::Lazy;
use serde_json::json;
use std::io;
use std::process::{ExitStatus, Stdio};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;

static PROJECT_ROOT: Lazy<String> =
    Lazy::new(|| std::env::var("BUCK_PROJECT_ROOT").expect("BUCK_PROJECT_ROOT must be set"));


/// Sends a filesystem mount request to the specified API endpoint
/// Parameters:
/// - repo: Repository path to mount
/// - mr: Merge request number
///   Returns: Result containing the response body string on success, or an error on failure
pub async fn mount_fs(repo: &str, mr: &str) -> Result<String, reqwest::Error> {
    // Create HTTP client
    let client = reqwest::Client::new();
    
    // Construct JSON request payload
    let payload = json!({
        "path": repo,
        "mr": mr,
    });

    // Send POST request
    let res = client
        .post("http://localhost:2725/api/fs/mount")
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()
        .await?;

    // Print status code
    println!("Mount request status: {}", res.status());
    
    // Get and return response body
    let body = res.text().await?;
    println!("Mount response body: {body}");
    
    Ok(body)
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

    let _ = mount_fs(&repo, &mr).await;

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
