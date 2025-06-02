use core::panic;
use http::Method;
use lazy_static::lazy_static;
use libfuse_fs::passthrough::newlogfs::LoggingFileSystem;
use scorpio::dicfuse::store;
use scorpio::fuse::MegaFuse;
use scorpio::manager::fetch::CheckHash;
use scorpio::manager::ScorpioManager;
use scorpio::server::mount_filesystem;
use scorpio::util::config;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::{env, fs, io};
use std::{ffi::OsStr, sync::Arc};
use testcontainers::core::wait::HttpWaitStrategy;
use testcontainers::{
    core::{IntoContainerPort, Mount, ReuseDirective, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum SCORCommand {
    ImportArc(),                // test init the scorpio directory structure
    WatchDir(),                 // update the directory structure
    LoadDir(String),            // test cd/ls and preload the directory structure
    GitAddFile(String, String), // add a new file and update to check the watch_dir
    GitDeleteFile(String),      // remove a new file and update to check the watch_dir
    Shutdown,                   // finish and close the file system service
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum CommandResult {
    StoreDirectoryStructure(HashMap<i32, BTreeSet<String>>), //used to return the directory structure
    Success,
    Error(String),
    InitFinish(usize), // used to indicate the initialization is finished
}

lazy_static! {
    static ref TARGET: String = {
        let mut manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // Get env at compile time
        manifest.pop();
        manifest.to_str().unwrap().to_string()
    };
    static ref MONO: PathBuf = {
        let path = if cfg!(target_os = "windows") {
            format!("{}/target/debug/mono.exe", TARGET.as_str())
        } else {
            format!("{}/target/debug/mono", TARGET.as_str())
        };
        PathBuf::from(path)
    };

    static ref SCOR_DIR: PathBuf = {
        Path::new("/tmp/scorpio_dir_test").to_path_buf()
    };

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

fn is_port_in_use(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port).parse().unwrap(),
        Duration::from_millis(1000),
    )
    .is_ok()
}

///clone the git repository and push to the mono server
fn git_clone(url: &str, mono_server_url: &str) -> io::Result<HashMap<i32, BTreeSet<String>>> {
    use std::collections::{BTreeSet, HashMap};

    fs::create_dir_all(SCOR_DIR.to_owned())?;

    let is_valid_git_repo = || -> bool {
        let git_config_path = SCOR_DIR.join("dir_test").join(".git");
        if !git_config_path.exists() {
            return false;
        }
        true
    };

    env::set_current_dir(SCOR_DIR.to_owned())?;

    if !is_valid_git_repo() {
        println!("No valid git repo found, cloning from {}", url);
        run_git_cmd(&["clone", url]);

        let repo_name = url.split('/').next_back().unwrap().trim_end_matches(".git");
        let repo_dir = SCOR_DIR.join(repo_name);
        env::set_current_dir(&repo_dir)?;

        let mono_url = format!("{}/third-party/dir_test.git", mono_server_url);
        run_git_cmd(&["remote", "add", "mono", mono_url.as_str()]);
        run_git_cmd(&["push", "--all", "mono"]);
    } else {
        println!("Using existing git repository");
        let repo_dir = SCOR_DIR.join("dir_test");

        env::set_current_dir(&repo_dir)?;
        let mono_url = format!("{}/third-party/dir_test.git", mono_server_url);
        run_git_cmd(&["remote", "remove", "mono"]);
        run_git_cmd(&["remote", "add", "mono", mono_url.as_str()]);
        run_git_cmd(&["push", "--all", "mono"]);
    }

    let mut depth_items: HashMap<i32, BTreeSet<String>> = HashMap::new();

    let output = Command::new("git").args(["ls-files"]).output()?;
    let files = String::from_utf8_lossy(&output.stdout);

    for file in files.lines() {
        let depth = file.chars().filter(|&c| c == '/').count() as i32;
        depth_items
            .entry(depth)
            .or_default()
            .insert(file.to_string());

        let parts: Vec<&str> = file.split('/').collect();
        for i in 0..parts.len() {
            if i == 0 {
                depth_items
                    .entry(0)
                    .or_default()
                    .insert(parts[0].to_string());
            } else {
                let parent_path = parts[0..i].join("/");
                let parent_depth = (i - 1) as i32;
                depth_items
                    .entry(parent_depth)
                    .or_default()
                    .insert(parent_path);
            }
        }
    }

    let config = format!(
        r#"
lfs_url = "{}"
store_path = "{}/store"
config_file = "config.toml"
git_author = "MEGA"
git_email = "admin@mega.org"
workspace = "{}/mount"
base_url = "{}"
dicfuse_readable = "true"
load_dir_depth = "4"
    "#,
        mono_server_url,
        SCOR_DIR.to_str().unwrap(),
        SCOR_DIR.to_str().unwrap(),
        mono_server_url,
    );

    let store_path = SCOR_DIR.join("store");
    let _ = fs::remove_dir_all(&store_path); // Clear old store
    let mount_path = SCOR_DIR.join("mount");
    let _ = fs::create_dir_all(&mount_path);
    let umount_result = Command::new("umount")
        .args(["-f", mount_path.to_str().unwrap()])
        .output();

    match umount_result {
        Ok(output) => {
            if !output.status.success() {
                println!(
                    "Umount warning: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        Err(e) => {
            println!("Umount command failed (this is usually okay): {}", e);
        }
    }

    fs::write(SCOR_DIR.join("scorpio.toml"), config)?;
    Ok(depth_items)
}
async fn mono_container(mapping_port: u16) -> ContainerAsync<GenericImage> {
    println!("MONO {:?} ", MONO.to_str().unwrap());
    if !MONO.exists() {
        panic!("MONO binary not found in \"target/debug/\", skip lfs test");
    }
    if is_port_in_use(mapping_port) {
        panic!("port {} is already in use", mapping_port);
    }
    let port_str = mapping_port.to_string();
    let cmd = vec![
        "/root/mono",
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
        .with_wait_for(WaitFor::Http(Box::new(
            HttpWaitStrategy::new("/")
                .with_method(Method::GET)
                .with_expected_status_code(404_u16),
        )))
        .with_mapped_port(mapping_port, mapping_port.tcp())
        .with_mount(Mount::bind_mount(MONO.to_str().unwrap(), "/root/mono"))
        .with_working_dir("/root")
        .with_reuse(ReuseDirective::Never)
        .with_cmd(cmd)
        .start()
        .await
        .expect("Failed to start mono_server")
}

pub async fn mono_bootstrap_servers(mapping_port: u16) -> (ContainerAsync<GenericImage>, String) {
    let container = mono_container(mapping_port).await;
    let mega_ip = container.get_bridge_ip_address().await.unwrap();
    let mega_port: u16 = container.get_host_port_ipv4(mapping_port).await.unwrap();
    (container, format!("http://{}:{}", mega_ip, mega_port))
}

#[tokio::test]
///Use container to run mono server and test the scorpio service
async fn test_scorpio_service_with_containers() {
    let (_container, mono_server_url) = mono_bootstrap_servers(12001).await;
    println!("container: {}", mono_server_url);
    let dir_list = git_clone("https://github.com/yyjeqhc/dir_test.git", &mono_server_url).unwrap();

    let (cmd_tx, cmd_rx) = mpsc::channel(32);
    let (result_tx, mut result_rx) = mpsc::channel(32);

    let scorpio_handle = tokio::spawn(test_scorpio_dir(cmd_rx, result_tx));

    // This is the preload's relative depth for the dir to load.
    let mut max_depth = 0;

    // Wait for the store:init_notify: Arc<Notify>
    tokio::select! {
        _ = scorpio_handle => {
            panic!("start the scorpio service failed");
        }
        success = result_rx.recv() => {
            if let Some(CommandResult::InitFinish(depth)) = success {
                   println!("scorpio service started successfully, max depth: {}", depth);
                   max_depth = depth;
            }
        }
    }
    println!("\n===== ImportArc =====");

    cmd_tx.send(SCORCommand::ImportArc()).await.unwrap();

    if let Some(result) = result_rx.recv().await {
        match result {
            CommandResult::StoreDirectoryStructure(store_items) => {
                for i in 0..max_depth as i32 {
                    assert_eq!(
                        dir_list.get(&i).unwrap(),
                        store_items.get(&i).unwrap(),
                        "dir structure at depth {} does not match.",
                        i
                    );
                }
                println!("import_arc,load dir success");
            }
            _ => {
                cmd_tx.send(SCORCommand::Shutdown).await.unwrap();
                let _ = result_rx.recv().await;
                // let _ = scorpio_handle.await;
                panic!("ImportArc failed to load dir.");
            }
        }
    }

    println!("\n===== add a file and WatchDir =====");
    let test_file = "test_file.txt";
    let test_content = format!(
        "now: {}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    cmd_tx
        .send(SCORCommand::GitAddFile(test_file.to_string(), test_content))
        .await
        .unwrap();

    if let Some(result) = result_rx.recv().await {
        match result {
            CommandResult::Success => {
                println!("git add success.");
            }
            _ => {
                cmd_tx.send(SCORCommand::Shutdown).await.unwrap();
                // let _ = scorpio_handle.await;
                let _ = result_rx.recv().await;

                panic!("git add file error.");
            }
        }
    }

    cmd_tx.send(SCORCommand::WatchDir()).await.unwrap();

    if let Some(result) = result_rx.recv().await {
        match result {
            CommandResult::StoreDirectoryStructure(result) => {
                assert!(
                    result.get(&0).unwrap().contains(&test_file.to_string()),
                    "WatchDir fail: did not find the added file"
                );
            }
            _ => {
                cmd_tx.send(SCORCommand::Shutdown).await.unwrap();
                // let _ = scorpio_handle.await;
                let _ = result_rx.recv().await;

                panic!("WatchDir error.");
            }
        }
    }

    println!("\n===== remove ad file and WatchDir =====");

    cmd_tx
        .send(SCORCommand::GitDeleteFile(test_file.to_string()))
        .await
        .unwrap();

    if let Some(result) = result_rx.recv().await {
        match result {
            CommandResult::Success => {
                println!("git remove success.");
            }
            _ => {
                cmd_tx.send(SCORCommand::Shutdown).await.unwrap();
                // let _ = scorpio_handle.await;
                let _ = result_rx.recv().await;

                panic!("git remove file error.");
            }
        }
    }

    cmd_tx.send(SCORCommand::WatchDir()).await.unwrap();

    if let Some(result) = result_rx.recv().await {
        match result {
            CommandResult::StoreDirectoryStructure(result) => {
                assert!(
                    !result.get(&0).unwrap().contains(&test_file.to_string()),
                    "WatchDir fail: did not remove the file"
                );
            }
            _ => {
                cmd_tx.send(SCORCommand::Shutdown).await.unwrap();
                // let _ = scorpio_handle.await;
                let _ = result_rx.recv().await;
                panic!("WatchDir: delete file error.");
            }
        }
    }

    cmd_tx
        .send(SCORCommand::LoadDir(
            "/third-party/dir_test/1/1/2/3".to_string(),
        ))
        .await
        .unwrap();

    if let Some(result) = result_rx.recv().await {
        match result {
            CommandResult::StoreDirectoryStructure(store_items) => {
                let mut expected_items: HashMap<i32, BTreeSet<String>> = HashMap::new();
                let test_path = "1/1/2/3/".to_string();
                for git_files in dir_list.values() {
                    for file in git_files {
                        if file.starts_with(&test_path) {
                            let relative_path = file.trim_start_matches(&test_path);
                            expected_items
                                .entry(relative_path.matches('/').count() as i32)
                                .or_default()
                                .insert(relative_path.to_string());
                        }
                    }
                }
                for i in 0..max_depth as i32 {
                    assert_eq!(
                        expected_items.get(&i).unwrap(),
                        store_items.get(&i).unwrap(),
                        "dir structure at depth {} does not match.",
                        i
                    );
                }
                println!("load_dir,preload dir success");
            }
            _ => {
                cmd_tx.send(SCORCommand::Shutdown).await.unwrap();
                let _ = result_rx.recv().await;
                panic!("load_dir fail");
            }
        }
    }

    cmd_tx.send(SCORCommand::Shutdown).await.unwrap();

    // let _ = scorpio_handle.await;
    let _ = result_rx.recv().await;

    println!("success to finish the test.");
}

async fn test_scorpio_dir(
    mut cmd_rx: mpsc::Receiver<SCORCommand>,
    result_tx: mpsc::Sender<CommandResult>,
) {
    if let Err(e) = config::init_config(SCOR_DIR.join("scorpio.toml").to_str().unwrap()) {
        eprintln!("init config fail {:?}", e);
        let _ = result_tx
            .send(CommandResult::Error("load config error".to_string()))
            .await;
        return;
    }

    let mut manager = ScorpioManager { works: vec![] };
    manager.check().await;
    //init scorpio configuration
    let fuse_interface = MegaFuse::new_from_manager(&manager).await;
    let workspace = config::workspace();
    let mountpoint = OsStr::new(workspace);
    let lgfs = LoggingFileSystem::new(fuse_interface.clone());

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let mut mount_handle = mount_filesystem(lgfs, mountpoint).await;
    let handle = &mut mount_handle;

    let arc_fuse = Arc::new(fuse_interface);
    let repo_dir = SCOR_DIR.join("dir_test");

    let shutdown_tx = shutdown_tx;
    let fuse_interface = arc_fuse.clone();
    let store = fuse_interface.dic.clone().store.clone();

    store.wait_for_ready().await;
    result_tx
        .send(CommandResult::InitFinish(store.max_depth()))
        .await
        .expect("Failed to send success signal");
    let cmd_handle = {
        let result_tx = result_tx.clone();

        tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    SCORCommand::ImportArc() => {
                        let base_path = "/third-party/dir_test";
                        let depth_items = store.get_dir_by_path(base_path).await;
                        if depth_items.is_empty() {
                            let _ = result_tx
                                .send(CommandResult::Error("ImportArc fail".to_string()))
                                .await;
                        } else {
                            let _ = result_tx
                                .send(CommandResult::StoreDirectoryStructure(depth_items))
                                .await;
                        }
                    }
                    SCORCommand::WatchDir() => {
                        store::watch_dir(store.clone()).await;
                        let dir_items = store.get_dir_by_path("/third-party/dir_test").await;
                        let _ = result_tx
                            .send(CommandResult::StoreDirectoryStructure(dir_items))
                            .await;
                    }
                    SCORCommand::LoadDir(path) => {
                        let max_depth = path.matches('/').count() + config::load_dir_depth();
                        store::load_dir(store.clone(), path.to_owned(), max_depth).await;
                        let dir_items = store.get_dir_by_path(&path).await;
                        if dir_items.is_empty() {
                            let _ = result_tx
                                .send(CommandResult::Error(format!("LoadDir fail: {}", path)))
                                .await;
                        } else {
                            let _ = result_tx
                                .send(CommandResult::StoreDirectoryStructure(dir_items))
                                .await;
                        }
                    }
                    SCORCommand::GitAddFile(path, content) => {
                        env::set_current_dir(&repo_dir).unwrap();
                        let _ = fs::write(repo_dir.join(&path), content);
                        run_git_cmd(&["add", &path]);
                        run_git_cmd(&["commit", "-m", "add file"]);
                        run_git_cmd(&["push", "--all", "mono"]);
                        result_tx.send(CommandResult::Success).await.unwrap();
                    }
                    SCORCommand::GitDeleteFile(path) => {
                        env::set_current_dir(&repo_dir).unwrap();
                        fs::remove_file(repo_dir.join(&path)).unwrap();
                        run_git_cmd(&["add", &path]);
                        run_git_cmd(&["commit", "-m", "remove file"]);
                        run_git_cmd(&["push", "--all", "mono"]);
                        result_tx.send(CommandResult::Success).await.unwrap();
                    }
                    SCORCommand::Shutdown => {
                        let _ = shutdown_tx.send(());
                        break;
                    }
                }
            }
        })
    };

    tokio::select! {
        res = handle => res.unwrap(),
        _ = cmd_handle => {
            println!("unmount....");
            mount_handle.unmount().await.unwrap();
            let _ = result_tx.send(CommandResult::Success).await;
        }
        _ = shutdown_rx => {
            println!("unmount....");
            mount_handle.unmount().await.unwrap();
            let _ = result_tx.send(CommandResult::Success).await;

        }
    }

    // let _ = cmd_handle.await;
    // let _ = daemon_handle.await;

    println!("success to close the scorpio service.");
}
