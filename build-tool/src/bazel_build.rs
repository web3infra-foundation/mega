use git2::Repository;
use std::{
    env, fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
};
use url::Url;

pub fn build() {
    let repo_path = PathBuf::from("/projects/mega");
    let project_name = repo_path.file_name().unwrap();

    let mut temp = PathBuf::from("/tmp/.mega/bazel_build_projects");
    temp.push(project_name);
    let mut project_url = Url::parse("http://localhost:8000").unwrap();
    project_url.set_path(repo_path.to_str().unwrap());
    if temp.exists() {
        if let Err(err) = fs::remove_dir_all(&temp) {
            tracing::error!("Error: {}", err);
        } else {
            tracing::info!("repo removed successfully: {:?}", project_name);
        }
    }
    Repository::clone(project_url.as_ref(), &temp).expect("failed to clone");

    if let Err(err) = env::set_current_dir(&temp) {
        tracing::error!("Failed to change the working directory: {}", err);
    } else {
        tracing::info!("Changed working directory to: {:?}", temp);

        // Execute cargo generate-lockfile command
        Command::new("cargo")
            .arg("generate-lockfile")
            .output()
            .unwrap();
        tracing::info!("project {:?} generate lockfile successfully", project_name);

        // Execute bazel sync crates command
        let mut sync_child = Command::new("bazel")
            .env("CARGO_BAZEL_REPIN", "1")
            .args(["sync", "--only=crate_index"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start bazel sync");

        let sync_stdout = sync_child.stdout.take().unwrap();
        for line in BufReader::new(sync_stdout).lines().flatten() {
            tracing::info!("project {:?} bazel sync: {}", project_name, line);
        }

        // Execute bazel build
        let mut bazel_build_child = Command::new("bazel")
            .args(["build", "//:mega"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start the bazel build");

        let build_stdout = bazel_build_child.stdout.take().unwrap();
        for line in BufReader::new(build_stdout).lines().flatten() {
            tracing::info!("project {:?} bazel build: {}", project_name, line);
        }
    }
}
