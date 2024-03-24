use std::{
    env,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    thread::{self, sleep},
    time::Duration,
};

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::StreamExt;
use git2::{build::RepoBuilder, Cred, FetchOptions, PushOptions, RemoteCallbacks, Repository};
use russh::{client, ChannelMsg};
use russh_keys::key;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::ToSocketAddrs;
use tokio_util::io::ReaderStream;

use gateway::ssh_server::load_key;
use git::{internal::pack::counter::GitTypeCounter, protocol::Protocol};

#[derive(Clone)]
pub struct P2pTestConfig {
    pub compose_path: String,
    pub pack_path: String,
    pub lifecycle_url: String,
    pub lifecycle_retrying: u64,
    pub repo_path: String,
    pub commit_id: String,
    pub sub_commit_id: String,
    pub counter: GitTypeCounter,
    pub clone_path: PathBuf,
    pub protocol: Protocol,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PackObjectIds {
    pub commit_ids: Vec<String>,
    pub tree_ids: Vec<String>,
    pub blob_ids: Vec<String>,
    pub tag_ids: Vec<String>,
}

// TODO: got some problem on copy content files
// pub fn build_image(config: &P2pTestConfig) {
//     let mut child = Command::new("docker")
//         .arg("compose")
//         .arg("-f")
//         .arg(&config.compose_path)
//         .arg("build")
//         .spawn()
//         .expect("Failed to execute command");
//     assert!(child.wait().is_ok());
// }

pub fn start_server(config: &P2pTestConfig) {
    let path = config.compose_path.clone();
    thread::spawn(move || {
        let mut child = Command::new("docker")
            .arg("compose")
            .arg("-f")
            .arg(path)
            .arg("up")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to execute command");
        // Wait for the child process to finish and get the result
        let _ = child.wait().expect("Failed to wait for child process");
    });
}

pub async fn lifecycle_check(config: &P2pTestConfig) {
    loop {
        let resp = reqwest::get(config.lifecycle_url.clone()).await.unwrap();
        if resp.status() == 200 {
            println!("lifecycle check passed");
            break;
        } else {
            println!(
                "lifecycle check failed, retrying in {} secs ...",
                config.lifecycle_retrying
            );
        }
        sleep(Duration::from_secs(config.lifecycle_retrying));
    }
}

pub async fn init_by_pack(config: &P2pTestConfig) {
    let pkt_line = format!("00980000000000000000000000000000000000000000 {} refs/heads/master\0 report-status-v2 side-band-64k agent=mega-test\n0000", config.commit_id);
    let f = tokio::fs::File::open(&config.pack_path).await.unwrap();

    match config.protocol {
        Protocol::Http | Protocol::P2p => {
            let stream =
                ReaderStream::new(Cursor::new(Bytes::from(pkt_line))).chain(ReaderStream::new(f));

            let client = reqwest::Client::new();
            let url = format!("http://localhost:8000{}/git-receive-pack", config.repo_path);
            let resp = client
                .post(url)
                .body(reqwest::Body::wrap_stream(stream))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), 200);
            println!("resp: {:?}", resp.bytes().await);
        }
        Protocol::Ssh => {
            // Create an asynchronous stream from the pkt_line string
            let pkt_line_stream = Cursor::new(pkt_line);
            // Combine the pkt_line and file streams
            let combined_stream = pkt_line_stream.chain(f);
            let mut ssh: Session = Session::connect("git".to_string(), ("localhost", 8100))
                .await
                .unwrap();
            let code = ssh
                .call(
                    &format!("git-receive-pack '{}'", config.repo_path),
                    combined_stream,
                )
                .await
                .unwrap();
            println!("Exitcode: {:?}", code);
        }
        _ => todo!(),
    }
}

pub fn clone_by_type(config: &P2pTestConfig, url: &str, into_path: &Path) -> Repository {
    match config.protocol {
        Protocol::Http => match Repository::clone(url, into_path) {
            Ok(repo) => repo,
            Err(e) => panic!("failed to clone: {}", e),
        },
        Protocol::Ssh => {
            // Create callbacks for SSH authentication
            let mut callbacks = RemoteCallbacks::new();

            callbacks.credentials(|_url, username_from_url, _allowed_types| {
                Cred::ssh_key(
                    username_from_url.unwrap(),
                    None,
                    Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
                    None,
                )
            });
            // Create fetch options
            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);

            // Clone the repository
            RepoBuilder::new()
                .fetch_options(fetch_options)
                .clone(url, into_path)
                .expect("Failed to clone repository")
        }
        _ => todo!(),
    }
}

pub fn push_by_type(config: &P2pTestConfig, repo: &Repository) {
    let mut remote = repo.find_remote("origin").unwrap();
    let refspecs = ["refs/heads/master:refs/heads/master"];

    let mut op = if config.protocol == Protocol::Ssh {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key(
                username_from_url.unwrap(),
                None,
                Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
                None,
            )
        });
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);
        Some(push_options)
    } else {
        None
    };
    remote.push(&refspecs, op.as_mut()).unwrap();
}

pub fn stop_server(config: &P2pTestConfig) {
    println!("stoping server and cleaning resources...");
    Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg(&config.compose_path)
        .arg("down")
        .spawn()
        .expect("Failed to execute command");
}

struct Client {}

// More SSH event handlers
// can be defined in this trait
// In this example, we're only using Channel, so these aren't needed.
#[async_trait]
impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// This struct is a convenience wrapper
/// around a russh client
pub struct Session {
    session: client::Handle<Client>,
}

impl Session {
    pub async fn connect<A: ToSocketAddrs>(user: impl Into<String>, addrs: A) -> Result<Self> {
        let key_pair = load_key()?;
        let config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(5)),
            ..<_>::default()
        };

        let config = Arc::new(config);
        let sh = Client {};

        let mut session = client::connect(config, addrs, sh).await?;
        let auth_res = session
            .authenticate_publickey(user, Arc::new(key_pair))
            .await?;

        if !auth_res {
            anyhow::bail!("Authentication failed");
        }

        Ok(Self { session })
    }

    pub async fn call<R: tokio::io::AsyncRead + Unpin>(
        &mut self,
        command: &str,
        data: R,
    ) -> Result<u32> {
        let mut channel = self.session.channel_open_session().await?;
        channel.exec(true, command).await?;
        // direct send pack to server regardless of the return
        channel.data(data).await?;
        channel.eof().await?;

        let mut code = 0;
        let mut stdout = tokio::io::stdout();

        loop {
            // There's an event available on the session channel
            let Some(msg) = channel.wait().await else {
                break;
            };
            match msg {
                // Write data to the terminal
                ChannelMsg::Data { ref data } => {
                    stdout.write_all(data).await?;
                    stdout.flush().await?;
                }
                // The command has returned an exit code
                ChannelMsg::ExitStatus { exit_status } => {
                    code = exit_status;
                    channel.eof().await?;
                    break;
                }
                _ => {}
            }
        }

        stdout
            .write_all(format!("exit code:{}\n", code).as_bytes())
            .await?;
        Ok(code)
    }
}
