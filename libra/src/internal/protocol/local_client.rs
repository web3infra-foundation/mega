use super::{
    DiscRef, FetchStream, ProtocolClient, generate_upload_pack_content, parse_discovered_references,
};
use bytes::Bytes;
use ceres::protocol::ServiceType;
use futures_util::stream::{self, StreamExt};
use mercury::errors::GitError;
use std::env;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use url::Url;

#[derive(Debug, Clone)]
pub struct LocalClient {
    repo_path: PathBuf,
}

impl ProtocolClient for LocalClient {
    fn from_url(url: &Url) -> Self {
        let path = url
            .to_file_path()
            .unwrap_or_else(|_| PathBuf::from(url.path()));
        Self { repo_path: path }
    }
}

impl LocalClient {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, IoError> {
        let path = path.as_ref();
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            env::current_dir()?.join(path)
        };
        if !absolute.exists() {
            return Err(IoError::other(format!(
                "Local repository path does not exist: {}",
                absolute.display()
            )));
        }
        let repo_path = if absolute.join("HEAD").exists() && absolute.join("objects").exists() {
            absolute
        } else if absolute.join(".git/HEAD").exists() {
            absolute.join(".git")
        } else {
            return Err(IoError::other(format!(
                "No valid Git directory structure found at: {}",
                absolute.display()
            )));
        };

        Ok(Self { repo_path })
    }

    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    pub async fn discovery_reference(
        &self,
        service: ServiceType,
    ) -> Result<Vec<DiscRef>, GitError> {
        if service != ServiceType::UploadPack {
            return Err(GitError::NetworkError(
                "Unsupported service type for local protocol".to_string(),
            ));
        }
        let output = Command::new("git-upload-pack")
            .arg("--advertise-refs")
            .arg(&self.repo_path)
            .output()
            .await
            .map_err(|e| {
                GitError::NetworkError(format!(
                    "Failed to spawn git-upload-pack for discovery: {}",
                    e
                ))
            })?;
        if !output.status.success() {
            return Err(GitError::NetworkError(format!(
                "git-upload-pack --advertise-refs failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let bytes = Bytes::from(output.stdout);
        parse_discovered_references(bytes, service)
    }

    pub async fn fetch_objects(
        &self,
        have: &[String],
        want: &[String],
    ) -> Result<FetchStream, IoError> {
        let body = generate_upload_pack_content(have, want);
        let mut child = Command::new("git-upload-pack");
        child.arg("--stateless-rpc");
        child.arg(&self.repo_path);
        child.stdin(std::process::Stdio::piped());
        child.stdout(std::process::Stdio::piped());
        child.stderr(std::process::Stdio::piped());
        let mut child = child
            .spawn()
            .map_err(|e| IoError::other(format!("Failed to spawn git-upload-pack: {e}")))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&body).await?;
        } else {
            return Err(IoError::other(
                "Failed to capture stdin for git-upload-pack process",
            ));
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| IoError::other(format!("Failed to wait for git-upload-pack: {e}")))?;
        if !output.status.success() {
            tracing::error!(
                "git-upload-pack stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return Err(IoError::other("git-upload-pack exited with failure"));
        }
        let stdout = Bytes::from(output.stdout);
        Ok(stream::once(async move { Ok(stdout) }).boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ceres::protocol::ServiceType;
    use std::ffi::OsStr;
    use std::process::Command as StdCommand;
    use tempfile::tempdir;
    use tokio::io::AsyncReadExt;
    use tokio_util::io::StreamReader;

    fn run_git<I, S>(cwd: Option<&Path>, args: I) -> StdCommand
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = StdCommand::new("git");
        if let Some(path) = cwd {
            cmd.current_dir(path);
        }
        cmd.args(args);
        cmd
    }

    #[tokio::test]
    async fn discovery_reference_empty_repo_returns_refs() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().join("empty.git");
        run_git(None, ["init", "--bare", repo_path.to_str().unwrap()])
            .status()
            .unwrap();

        let client = LocalClient::from_path(&repo_path).unwrap();
        let refs = client
            .discovery_reference(ServiceType::UploadPack)
            .await
            .unwrap();
        assert!(refs.is_empty());
    }

    #[tokio::test]
    async fn fetch_objects_produces_pack_stream() {
        let temp = tempdir().unwrap();
        let remote_path = temp.path().join("remote.git");
        let work_path = temp.path().join("work");

        assert!(
            run_git(None, ["init", "--bare", remote_path.to_str().unwrap()])
                .status()
                .unwrap()
                .success()
        );
        assert!(
            run_git(None, ["init", work_path.to_str().unwrap()])
                .status()
                .unwrap()
                .success()
        );
        assert!(
            run_git(Some(&work_path), ["config", "user.name", "Local Tester"])
                .status()
                .unwrap()
                .success()
        );
        assert!(
            run_git(Some(&work_path), ["config", "user.email", "local@test"])
                .status()
                .unwrap()
                .success()
        );
        std::fs::write(work_path.join("README.md"), "hello world").unwrap();
        assert!(
            run_git(Some(&work_path), ["add", "README.md"])
                .status()
                .unwrap()
                .success()
        );
        assert!(
            run_git(Some(&work_path), ["commit", "-m", "initial commit"])
                .status()
                .unwrap()
                .success()
        );

        let branch = String::from_utf8(
            run_git(Some(&work_path), ["rev-parse", "--abbrev-ref", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();

        assert!(
            run_git(
                Some(&work_path),
                ["remote", "add", "origin", remote_path.to_str().unwrap()],
            )
            .status()
            .unwrap()
            .success()
        );
        assert!(
            run_git(
                Some(&work_path),
                ["push", "origin", &format!("HEAD:refs/heads/{branch}"),],
            )
            .status()
            .unwrap()
            .success()
        );

        let head = String::from_utf8(
            run_git(Some(&work_path), ["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();

        let client = LocalClient::from_path(&remote_path).unwrap();
        let refs = client
            .discovery_reference(ServiceType::UploadPack)
            .await
            .unwrap();
        assert!(!refs.is_empty());

        let want = vec![head];
        let have = Vec::new();
        let stream = client.fetch_objects(&have, &want).await.unwrap();
        let mut reader = StreamReader::new(stream);
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await.unwrap();
        assert!(buf.windows(4).any(|w| w == b"PACK"));
    }
}
