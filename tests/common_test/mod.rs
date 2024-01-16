use std::{
    env,
    io::Cursor,
    process::Command,
    thread::{self, sleep},
    time::Duration, path::PathBuf,
};

use bytes::Bytes;
use futures_util::StreamExt;
use git2::Repository;
use serde::{Deserialize, Serialize};
use tokio_util::io::ReaderStream;

use git::internal::pack::counter::GitTypeCounter;

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
    // docker compose -f tests/compose/mega_p2p/compose.yaml up --build
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
    let mut source = env::current_dir().unwrap();
    source.push(&config.pack_path);
    let pkt_line = format!("00980000000000000000000000000000000000000000 {} refs/heads/master\0 report-status-v2 side-band-64k agent=mega-test\n0000", config.commit_id);

    let f = tokio::fs::File::open(source).await.unwrap();
    let stream = ReaderStream::new(Cursor::new(Bytes::from(pkt_line))).chain(ReaderStream::new(f));
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


pub fn git2_clone(url: &str, into_path: &str) {
    match Repository::clone(url, into_path) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to clone: {}", e),
    };
}

pub fn get_last_commit_id(repo_path: &str) -> String {
    let repository = Repository::open(repo_path).expect("Failed to open repository");
    let head = repository.head().expect("Failed to get HEAD reference");
    head.target().unwrap().to_string()
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
