use std::{
    process::Command,
    thread::{self, sleep},
    time::Duration,
};

#[derive(Clone)]
pub struct P2pTestConfig {
    pub compose_path: String,
    pub lifecycle_url: String,
    pub lifecycle_retrying: u64,
}
pub async fn init_p2p_server(config: P2pTestConfig) {
    // docker compose -f tests/compose/mega_p2p/compose.yaml up --build
    let P2pTestConfig {
        compose_path,
        lifecycle_url,
        lifecycle_retrying,
    } = config;
    thread::spawn(move || {
        let mut child = Command::new("docker")
            .arg("compose") // Provide arguments if needed
            .arg("-f")
            .arg(compose_path)
            .arg("up")
            .stdin(std::process::Stdio::piped()) // Pass the standard input stream as an argument
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to execute command");
        // Wait for the child process to finish and get the result
        let _ = child.wait().expect("Failed to wait for child process");
    });

    loop {
        let resp = reqwest::get(&lifecycle_url).await.unwrap();
        if resp.status() == 200 {
            break;
        } else {
            println!(
                "lifecycle check failed, retrying in {} secs ...",
                lifecycle_retrying
            );
        }
        sleep(Duration::from_secs(lifecycle_retrying));
    }
}

pub fn provide_data_before_test() {
    let res = Command::new("git")
        .arg("remote")
        .arg("set-url")
        .arg("local")
        .arg("http://localhost:8000/projects/mega.git")
        .output()
        .expect("Failed to execute command");
    assert!(res.status.success());
    let res2 = Command::new("git")
        .arg("push")
        .arg("local")
        .arg("main")
        .output()
        .expect("Failed to execute command");
    assert!(res2.status.success());
}

pub fn stop_p2p_server(config: P2pTestConfig) {
    let P2pTestConfig {
        compose_path,
        lifecycle_url: _,
        lifecycle_retrying: _,
    } = config;
    println!("stoping p2p server and cleaning resources...");
    Command::new("docker")
        .arg("compose") // Provide arguments if needed
        .arg("-f")
        .arg(compose_path)
        .arg("down")
        .output()
        .expect("Failed to execute command");
}
