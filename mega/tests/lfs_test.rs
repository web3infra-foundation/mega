mod common;

/// integration tests for the mega module
use std::process::{Child, Command};
use std::{env, fs, io, thread};
use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;
use rand::Rng;
use serial_test::serial;
use tempfile::TempDir;
use lazy_static::lazy_static;

const PORT: u16 = 8000; // mega server port
const LARGE_FILE_SIZE_MB: usize = 60;

lazy_static! {
    static ref LFS_URL: String = format!("http://localhost:{}", PORT);

    static ref TARGET: String = {
        // mega/mega, absolute
        let mut manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // Get env at compile time
        manifest.pop(); // remove "mega" from path
        manifest.join("target").to_str().unwrap().to_string()
    };

    static ref LIBRA: PathBuf = {
        let path = format!("{}/debug/libra", TARGET.as_str());
        PathBuf::from(path)
    };

    static ref MEGA: PathBuf = {
        let path = format!("{}/debug/mega", TARGET.as_str());
        PathBuf::from(path)
    };
}

/// check if git lfs is installed
fn check_git_lfs() -> bool {
    let status = Command::new("git")
        .args(["lfs", "version"])
        .status()
        .expect("Failed to execute git lfs version");

    status.success()
}

fn run_cmd(program: &str, args: &[&str], stdin: Option<&str>, envs: Option<Vec<(&str, &str)>>) {
    let mut cmd = assert_cmd::Command::new(program);
    let mut cmd = cmd.args(args);
    if let Some(stdin) = stdin {
        cmd = cmd.write_stdin(stdin);
    }
    if let Some(envs) = envs {
        cmd = cmd.envs(envs);
    }
    let assert = cmd.assert().success();
    let output = assert.get_output();

    println!("Command success: {} {}\nStatus: {}\nStdout: {}",
        program,
        args.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stdout),
    );
}

fn run_git_cmd(args: &[&str]) {
    run_cmd("git", args, None, None);
}

fn run_libra_cmd(args: &[&str]) {
    run_cmd(LIBRA.to_str().unwrap(), args, None, None);
}

fn run_libra_cmd_with_stdin(args: &[&str], stdin: Option<&str>, envs: Option<Vec<(&str, &str)>>) {
    run_cmd(LIBRA.to_str().unwrap(), args, stdin, envs);
}

fn is_port_in_use(port: u16) -> bool {
    TcpStream::connect_timeout(&format!("127.0.0.1:{}", port).parse().unwrap(), Duration::from_millis(1000))
        .is_ok()
}

/// Run mega server in a new process
fn run_mega_server(data_dir: &Path) -> Child {
    if !MEGA.exists() {
        panic!("mega binary not found in \"target/debug/\", skip lfs test");
    }
    if is_port_in_use(PORT) {
        panic!("port {} is already in use", PORT);
    }

    // env var can be shared between parent and child process
    env::set_var("MEGA_BASE_DIR", data_dir);

    let server = Command::new(MEGA.to_str().unwrap())
        .args(["service", "multi", "http"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .spawn()
        .expect("Failed to start mega server");

    // loop check until port to be ready
    let mut i = 0;
    while !is_port_in_use(PORT) && (i < 15) {
        thread::sleep(Duration::from_secs(1));
        i += 1;
    }
    assert!(is_port_in_use(PORT), "mega server not started");
    println!("mega server started in {} secs", i);
    thread::sleep(Duration::from_secs(1));

    server
}

fn generate_large_file(path: &str, size_mb: usize) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    let mut rng = rand::rng();

    const BUFFER_SIZE: usize = 1024 * 1024; // 1MB buffer
    let mut buffer = [0u8; BUFFER_SIZE];

    for _ in 0..size_mb {
        rng.fill(&mut buffer[..]);
        file.write_all(&buffer)?;
    }

    Ok(())
}

fn git_lfs_push(url: &str) -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_str().unwrap();
    println!("repo_path: {}", repo_path);

    // git init
    run_git_cmd(&["init", repo_path]);

    env::set_current_dir(repo_path)?;

    // track Large file
    run_git_cmd(&["lfs", "track", "*.bin"]);
    // create large file
    generate_large_file("large_file.bin", LARGE_FILE_SIZE_MB)?;
    // add & commit
    run_git_cmd(&["add", "."]);
    run_git_cmd(&["commit", "-m", "add large file"]);

    run_git_cmd(&["remote", "add", "mega", url]);

    // set lfs.url
    run_git_cmd(&["config", "lfs.url", &LFS_URL]);
    // push to mega server
    run_git_cmd(&["push", "--all", "mega"]);

    Ok(())
}

fn libra_lfs_push(url: &str) -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_str().unwrap();
    println!("repo_path: {}", repo_path);

    env::set_current_dir(repo_path)?;

    // libra init
    run_libra_cmd(&["init"]);
    // track Large file
    run_libra_cmd(&["lfs", "track", "*.bin"]);
    // create large file
    generate_large_file("large_file.bin", LARGE_FILE_SIZE_MB)?;
    // add & commit
    run_libra_cmd(&["add", "."]);
    run_libra_cmd(&["commit", "-m", "add large file"]);
    // add remote
    run_libra_cmd(&["remote", "add", "mega", url]);
    // branch --set-upstream-to=origin/master
    run_libra_cmd(&["branch", "--set-upstream-to=mega/master"]);
    // try lock API
    run_libra_cmd(&["lfs", "lock", "large_file.bin"]);
    // push to mega server
    run_libra_cmd_with_stdin(&["push", "mega", "master"],
                             Some("mega\nmega"), // basic auth, can be overridden by env var
                             Some(vec![("LIBRA_NO_HIDE_PASSWORD", "1")]));

    Ok(())
}

fn git_lfs_clone(url: &str) -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    println!("clone temp_dir: {:?}", temp_dir.path());
    env::set_current_dir(temp_dir.path())?;
    // git clone url
    // `--config`: temporary set lfs.url
    run_git_cmd(&["clone", url, "--config", &("lfs.url=".to_owned() + &LFS_URL)]);

    let file = Path::new("lfs/large_file.bin");
    assert!(file.exists(), "Failed to clone large file");
    assert_eq!(file.metadata()?.len(), LARGE_FILE_SIZE_MB as u64 * 1024 * 1024);
    Ok(())
}

fn libra_lfs_clone(url: &str) -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    println!("clone temp_dir: {:?}", temp_dir.path());
    env::set_current_dir(temp_dir.path())?;
    // libra clone url
    run_libra_cmd(&["clone", url]);

    let file = Path::new("lfs-libra/large_file.bin");
    assert!(file.exists(), "Failed to clone large file");
    assert_eq!(file.metadata()?.len(), LARGE_FILE_SIZE_MB as u64 * 1024 * 1024);
    Ok(())
}

#[test]
#[serial]
fn lfs_split_with_git() {
    assert!(check_git_lfs(), "git lfs is not installed");

    let mega_dir = TempDir::new().unwrap();
    env::set_var("MEGA_authentication__enable_http_auth", "false"); // no need for git
    // start mega server at background (new process)
    let mut mega = run_mega_server(mega_dir.path());

    let url = &format!("http://localhost:{}/third-part/lfs.git", PORT);
    git_lfs_push(url).expect("Failed to push large file to mega server");
    git_lfs_clone(url).expect("Failed to clone large file from mega server");

    mega.kill().expect("Failed to kill mega server");
    let _ = mega.wait();
    thread::sleep(Duration::from_secs(1)); // wait for server to stop, avoiding affecting other tests
}

#[test]
#[serial]
fn lfs_split_with_libra() {
    if !LIBRA.exists() {
        panic!("libra binary not found in \"target/debug/\", skip lfs test");
    }

    let mega_dir = TempDir::new().unwrap();
    env::set_var("MEGA_authentication__enable_http_auth", "true");
    env::set_var("MEGA_authentication__enable_test_user", "true");
    env::set_var("MEGA_authentication__test_user_name", "mega");
    env::set_var("MEGA_authentication__test_user_token", "mega");
    // start mega server at background (new process)
    let mut mega = run_mega_server(mega_dir.path());

    let url = &format!("http://localhost:{}/third-part/lfs-libra.git", PORT);
    libra_lfs_push(url).expect("(libra)Failed to push large file to mega server");
    libra_lfs_clone(url).expect("(libra)Failed to clone large file from mega server");

    env::set_var("MEGA_authentication__enable_http_auth", "false"); // avoid affecting other tests
    mega.kill().expect("Failed to kill mega server");
    let _ = mega.wait();
    thread::sleep(Duration::from_secs(1)); // wait for server to stop, avoiding affecting other tests
}