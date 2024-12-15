use std::io;
use std::process::{Command, Stdio};
use crate::util;

const PROJECT_ROOT: &str = "/home/bean/projects/buck2";

pub fn build(repo: String, target: String, args: Vec<String>, log_path: String) -> io::Result<String> {
    util::ensure_parent_dirs(&log_path)?;
    let output_file = std::fs::File::create(log_path)?;

    let mut cmd = Command::new("buck2");
    let cmd = cmd
        .arg("build")
        .args(args)
        .arg(target)
        .current_dir(&format!("{}/{}", PROJECT_ROOT, repo))
        .stdout(Stdio::from(output_file.try_clone()?))
        .stderr(Stdio::from(output_file));
    // actually, some info (like: "BUILD SUCCESSFUL") is printed to stderr

    tracing::debug!("cmd:{:?}", cmd);

    let mut child = cmd.spawn()?;
    child.wait()?;

    Ok("BUILD ENDED".to_string())
}