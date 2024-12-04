use std::process::Command;

const PROJECT_ROOT: &str = "/home/bean/projects/buck2";

pub fn build(repo: String, target: String, args: Vec<String>) -> Result<String, String> {
    let mut cmd = Command::new("buck2");
    let cmd = cmd
        .arg("build")
        .args(args)
        .arg(target)
        .current_dir(&format!("{}/{}", PROJECT_ROOT, repo));

    tracing::debug!("cmd:{:?}", cmd);

    let output = cmd.output().map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    // actually, some info (like: "BUILD SUCCESSFUL") is printed to stderr
    let output = String::from_utf8_lossy(&output.stdout);
    Ok(output.to_string())
}