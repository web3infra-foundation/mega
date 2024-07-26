mod common;

/// integration tests for the mega module
use std::process::Command;
use std::{env, fs, io, thread};
use std::io::Write;
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;
use rand::Rng;
use tempfile::TempDir;

const PORT: u16 = 8000; // mega server port
/// check if git lfs is installed
fn check_git_lfs() -> bool {
    let status = Command::new("git")
        .args(["lfs", "version"])
        .status()
        .expect("Failed to execute git lfs version");

    status.success()
}

fn run_git_cmd(args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .status()
        .unwrap();

    assert!(status.success(), "Git command failed: git {}", args.join(" "));
}

fn is_port_in_use(port: u16) -> bool {
    TcpStream::connect_timeout(&format!("127.0.0.1:{}", port).parse().unwrap(), Duration::from_millis(1000))
        .is_ok()
}

fn run_mega_server() {
    thread::spawn(|| {
        let args = vec!["service", "multi", "http"];
        mega::cli::parse(Some(args)).expect("Failed to start mega service");
    });
    // loop check until port to be ready
    let mut i = 0;
    while !is_port_in_use(PORT) && (i < 15) {
        thread::sleep(Duration::from_secs(1));
        i += 1;
    }
    assert!(is_port_in_use(PORT), "mega server not started");
    println!("mega server started in {} secs", i);
}

fn generate_large_file(path: &str, size_mb: usize) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    let mut rng = rand::thread_rng();

    const BUFFER_SIZE: usize = 1024 * 1024; // 1MB buffer
    let mut buffer = [0u8; BUFFER_SIZE];

    for _ in 0..size_mb {
        rng.fill(&mut buffer[..]);
        file.write_all(&buffer)?;
    }

    Ok(())
}

fn lfs_push(url: &str) -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_str().unwrap();
    println!("repo_path: {}", repo_path);

    // git init
    run_git_cmd(&["init", repo_path]);

    env::set_current_dir(repo_path)?;

    // track Large file
    run_git_cmd(&["lfs", "track", "*.bin"]);

    // create large file
    generate_large_file("large_file.bin", 60)?;

    // add & commit
    run_git_cmd(&["add", "."]);
    run_git_cmd(&["commit", "-m", "add large file"]);

    // push to mega server
    run_git_cmd(&["remote", "add", "mega", url]);
    run_git_cmd(&["push", "--all", "mega"]);

    Ok(())
}

fn lfs_clone(url: &str) -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    println!("clone temp_dir: {:?}", temp_dir.path());
    env::set_current_dir(temp_dir.path())?;
    // git clone url
    run_git_cmd(&["clone", url]);

    assert!(Path::new("lfs/large_file.bin").exists(), "Failed to clone large file");
    Ok(())
}

#[test]
fn lfs_split_with_git() {
    assert!(check_git_lfs(), "git lfs is not installed");

    let mega_dir = TempDir::new().unwrap();
    env::set_var("MEGA_BASE_DIR", mega_dir.path());
    // start mega server at background
    run_mega_server();

    let url = &format!("http://localhost:{}/third-part/lfs.git", PORT);
    lfs_push(url).expect("Failed to push large file to mega server");
    lfs_clone(url).expect("Failed to clone large file from mega server");
}