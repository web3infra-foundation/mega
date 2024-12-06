use std::process::{Command, Stdio};

const PROJECT_ROOT: &str = "/home/bean/projects/buck2";

pub fn build(repo: String, target: String, args: Vec<String>, id: String) -> std::io::Result<String> {
    const BUILD_LOG_DIR: &str = "/tmp/buck2ctl";
    std::fs::create_dir_all(BUILD_LOG_DIR)?;
    let output_file = std::fs::File::create(&format!("{}/{}", BUILD_LOG_DIR, id))?;

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

    Ok("BUILD SUCCEEDED".to_string())
}