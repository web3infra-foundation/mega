use std::{
    env,
    process::Command,
    thread::{self, sleep},
    time::Duration,
};

use bytes::Bytes;
use futures_util::StreamExt;
use tokio_util::io::ReaderStream;

#[derive(Clone)]
pub struct P2pTestConfig {
    pub compose_path: String,
    pub pack_path: String,
    pub lifecycle_url: String,
    pub lifecycle_retrying: u64,
    pub repo_name: String,
    pub commit_id: String,
    pub obj_num: i32,
}

pub fn build_image(config: &P2pTestConfig) {
    let child = Command::new("docker")
        .arg("compose") // Provide arguments if needed
        .arg("-f")
        .arg(&config.compose_path)
        .arg("build")
        .output()
        .expect("Failed to execute command");
    assert!(child.status.success());
}

pub fn start_server(config: &P2pTestConfig) {
    let path = config.compose_path.clone();
    // docker compose -f tests/compose/mega_p2p/compose.yaml up --build
    thread::spawn(move || {
        let mut child = Command::new("docker")
            .arg("compose") // Provide arguments if needed
            .arg("-f")
            .arg(path)
            .arg("up")
            .stdin(std::process::Stdio::piped()) // Pass the standard input stream as an argument
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

// pub fn init_by_command() {
//     let res = Command::new("git")
//         .arg("remote")
//         .arg("set-url")
//         .arg("local")
//         .arg("http://localhost:8000/projects/mega.git")
//         .output()
//         .expect("Failed to execute command");
//     assert!(res.status.success());
//     let res2 = Command::new("git")
//         .arg("push")
//         .arg("local")
//         .arg("main")
//         .output()
//         .expect("Failed to execute command");
//     assert!(res2.status.success());
// }

pub async fn init_by_pack(config: &P2pTestConfig) {
    let mut source = env::current_dir().unwrap();
    source.push(&config.pack_path);
    let pkt_line = format!("00980000000000000000000000000000000000000000 {} refs/heads/master\0 report-status-v2 side-band-64k agent=mega-test\n0000", config.commit_id);
    let pkt_line = std::io::Cursor::new(Bytes::from(pkt_line));

    let f = tokio::fs::File::open(source).await.unwrap();
    let stream = ReaderStream::new(pkt_line).chain(ReaderStream::new(f));
    let client = reqwest::Client::new();
    let url = format!(
        "http://localhost:8000/projects/{}/git-receive-pack",
        config.repo_name
    );
    let resp = client
        .post(url)
        .body(reqwest::Body::wrap_stream(stream))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    println!("resp: {:?}", resp.bytes().await);

    //TODO: check objenums matchs
}

pub fn stop_server(config: &P2pTestConfig) {
    println!("stoping server and cleaning resources...");
    Command::new("docker")
        .arg("compose") // Provide arguments if needed
        .arg("-f")
        .arg(&config.compose_path)
        .arg("down")
        .output()
        .expect("Failed to execute command");
}
