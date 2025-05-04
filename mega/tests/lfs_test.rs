mod common;

use http::Method;
use lazy_static::lazy_static;
use rand::Rng;
use serial_test::serial;
use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
/// integration tests for the mega module
use std::process::Command;
use std::time::Duration;
use std::{env, fs, io, thread};
use tempfile::TempDir;
use testcontainers::core::wait::HttpWaitStrategy;
use testcontainers::{
    core::{IntoContainerPort, ReuseDirective, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};
const LARGE_FILE_SIZE_MB: usize = 60;

struct ChildGuard(std::process::Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
        println!("mega killed in Drop");
    }
}

lazy_static! {
    // static ref LFS_URL: String =

    static ref TARGET: String = {
        // mega/mega, absolute
        let mut manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // Get env at compile time
        manifest.pop();
        manifest.to_str().unwrap().to_string()
    };

    static ref LIBRA: PathBuf = {
        let path = if cfg!(target_os = "windows") {
            format!("{}/target/debug/libra.exe", TARGET.as_str())
        } else {
            format!("{}/target/debug/libra", TARGET.as_str())
        };
        PathBuf::from(path)
    };

    static ref MEGA: PathBuf = {
        let path = if cfg!(target_os = "windows") {
            format!("{}/target/debug/mega.exe", TARGET.as_str())
        } else {
            format!("{}/target/debug/mega", TARGET.as_str())
        };
        PathBuf::from(path)
    };

    static ref CONFIG: PathBuf = {
        let path =  format!("{}/mega/config.toml",TARGET.as_str());
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

    println!(
        "Command success: {} {}\nStatus: {}\nStdout: {}",
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
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port).parse().unwrap(),
        Duration::from_millis(1000),
    )
    .is_ok()
}

/// Run mega server in a new process
fn run_mega_server(data_dir: &Path, http_port: u16) -> ChildGuard {
    if !MEGA.exists() {
        panic!("mega binary not found in \"target/debug/\", skip lfs test");
    }
    if is_port_in_use(http_port) {
        panic!("port {} is already in use", http_port);
    }

    // env var can be shared between parent and child process
    env::set_var("MEGA_BASE_DIR", data_dir);

    let server = Command::new(MEGA.to_str().unwrap())
        .args(["service", "multi", "http", "-p", &format!("{}", http_port)])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .spawn()
        .expect("Failed to start mega server");

    // loop check until port to be ready
    let mut i = 0;
    while !is_port_in_use(http_port) && (i < 15) {
        thread::sleep(Duration::from_secs(1));
        i += 1;
    }
    assert!(is_port_in_use(http_port), "mega server not started");
    println!("mega server started in {} secs", i);
    thread::sleep(Duration::from_secs(1));

    // server
    ChildGuard(server)
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

fn git_lfs_push(url: &str, lfs_url: &str) -> io::Result<()> {
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
    run_git_cmd(&["config", "lfs.url", lfs_url]);
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
    run_libra_cmd_with_stdin(
        &["push", "mega", "master"],
        Some("mega\nmega"), // basic auth, can be overridden by env var
        Some(vec![("LIBRA_NO_HIDE_PASSWORD", "1")]),
    );

    Ok(())
}

fn git_lfs_clone(url: &str, lfs_url: &str) -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    println!("clone temp_dir: {:?}", temp_dir.path());
    env::set_current_dir(temp_dir.path())?;
    // git clone url
    // `--config`: temporary set lfs.url
    run_git_cmd(&["clone", url, "--config", &format!("lfs.url={}", lfs_url)]);

    let file = Path::new("lfs/large_file.bin");
    assert!(file.exists(), "Failed to clone large file");
    assert_eq!(
        file.metadata()?.len(),
        LARGE_FILE_SIZE_MB as u64 * 1024 * 1024
    );
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
    assert_eq!(
        file.metadata()?.len(),
        LARGE_FILE_SIZE_MB as u64 * 1024 * 1024
    );
    Ok(())
}

#[test]
#[ignore]
#[serial]
//Use containes insted.
fn lfs_split_with_git() {
    assert!(check_git_lfs(), "git lfs is not installed");

    let mega_dir = TempDir::new().unwrap();
    env::set_var("MEGA_authentication__enable_http_auth", "false"); // no need for git
                                                                    // start mega server at background (new process)
    let mega = run_mega_server(mega_dir.path(), 58001);
    let mega_start_port = 58001;
    let url = &format!("http://localhost:{}/third-party/lfs.git", mega_start_port);
    let lfs_url = format!("http://localhost:{}", mega_start_port);
    let push_result = git_lfs_push(url, &lfs_url);
    let clone_result = git_lfs_clone(url, &lfs_url);

    println!("{:?}", mega.0);

    push_result.expect("Failed to push large file to mega server");
    clone_result.expect("Failed to clone large file from mega server");
    thread::sleep(Duration::from_secs(1)); // wait for server to stop, avoiding affecting other tests
}

#[test]
#[serial]
#[ignore]
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
    let mega = run_mega_server(mega_dir.path(), 58002);

    let url = &format!("http://localhost:{}/third-party/lfs-libra.git", 58002);
    let push_result = libra_lfs_push(url);
    let clone_result = libra_lfs_clone(url);

    env::set_var("MEGA_authentication__enable_http_auth", "false"); // avoid affecting other tests
    println!("{:?}", mega.0);

    push_result.expect("Failed to push large file to mega server");
    clone_result.expect("Failed to clone large file from mega server");
    thread::sleep(Duration::from_secs(1)); // wait for server to stop, avoiding affecting other tests
}

async fn mega_container(mapping_port: u16) -> ContainerAsync<GenericImage> {
    println!("MEGA {:?} ", MEGA.to_str().unwrap());
    println!("CONFIG {:?} ", CONFIG.to_str().unwrap());
    if !MEGA.exists() {
        panic!("mega binary not found in \"target/debug/\", skip lfs test");
    }
    if is_port_in_use(mapping_port) {
        panic!("port {} is already in use", mapping_port);
    }
    let port_str = mapping_port.to_string();
    let cmd = vec![
        "/root/mega",
        "service",
        "multi",
        "http",
        "-p",
        &port_str,
        "--host",
        "0.0.0.0",
    ];

    GenericImage::new("ubuntu", "latest")
        .with_exposed_port(mapping_port.tcp())
        .with_wait_for(WaitFor::Http(
            HttpWaitStrategy::new("/")
                .with_method(Method::GET)
                .with_expected_status_code(404_u16),
        ))
        .with_mapped_port(mapping_port, mapping_port.tcp())
        .with_copy_to("/root/mega", MEGA.clone())
        .with_copy_to("/root/config.toml", CONFIG.clone())
        .with_env_var("MEGA_authentication__enable_http_auth", "false")
        .with_working_dir("/root")
        .with_reuse(ReuseDirective::Never)
        .with_cmd(cmd)
        .start()
        .await
        .expect("Failed to start mega_server")
}

pub async fn mega_bootstrap_servers(mapping_port: u16) -> (ContainerAsync<GenericImage>, String) {
    let container = mega_container(mapping_port).await;
    let mega_ip = container.get_bridge_ip_address().await.unwrap();
    let mega_port: u16 = container.get_host_port_ipv4(mapping_port).await.unwrap();
    (container, format!("http://{}:{}", mega_ip, mega_port))
}

#[tokio::test]
///Use container to run mega server and test lfs_split
async fn test_lfs_split_with_containers() {
    let (_container, mega_server_url) = mega_bootstrap_servers(10000).await;
    println!("container: {}", mega_server_url);

    let url = &format!("{}/third-party/lfs.git", mega_server_url);
    let lfs_url = mega_server_url;
    let push_result = git_lfs_push(url, &lfs_url);
    let clone_result = git_lfs_clone(url, &lfs_url);

    push_result.expect("Failed to push large file to mega server");
    clone_result.expect("Failed to clone large file from mega server");
    thread::sleep(Duration::from_secs(1)); // wait for server to stop, avoiding affecting other tests
}
