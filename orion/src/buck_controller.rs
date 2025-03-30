use crate::util;
use crate::ws::WSMessage;
use std::io;
use std::process::{ExitStatus, Stdio};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;

const PROJECT_ROOT: &str = "/home/bean/projects/buck2";

pub async fn build(
    id: String,
    repo: String,
    target: String,
    args: Vec<String>,
    // log_path: String,
    sender: UnboundedSender<WSMessage>,
) -> io::Result<ExitStatus> {
    // util::ensure_parent_dirs(&log_path)?;
    // let output_file = std::fs::File::create(log_path)?;

    let mut cmd = Command::new("buck2");
    let cmd = cmd
        .arg("build")
        .args(args)
        .arg(target)
        .current_dir(format!("{}/{}", PROJECT_ROOT, repo))
        // .stdout(output_file.try_clone()?)
        // .stderr(output_file);
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
                            build_id: id.clone(),
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
                            build_id: id.clone(),
                            output: line.clone(),
                        }).unwrap();
                    },
                    Err(_) => break,
                    _ => (),
                }
            }
            result = child.wait() => {
                return Ok(result?);
            }
        }
    }

    let status = child.wait().await?;
    Ok(status)
}
