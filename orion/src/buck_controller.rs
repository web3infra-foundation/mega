use crate::ws::WSMessage;
use once_cell::sync::Lazy;
use std::io;
use std::process::{ExitStatus, Stdio};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;

static PROJECT_ROOT: Lazy<String> =
    Lazy::new(|| std::env::var("BUCK_PROJECT_ROOT").expect("BUCK_PROJECT_ROOT must be set"));

pub async fn build(
    id: String,
    repo: String,
    target: String,
    args: Vec<String>,
    sender: UnboundedSender<WSMessage>,
) -> io::Result<ExitStatus> {
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
