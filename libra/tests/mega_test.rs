use http::Method;
use lazy_static::lazy_static;
use libra::internal::protocol::lfs_client::LFSClient;
use libra::internal::protocol::ProtocolClient;
use libra::utils::lfs;
use reqwest::Url;
use std::env;
use std::net::TcpStream;
use std::path::PathBuf;
use testcontainers::core::wait::HttpWaitStrategy;
use testcontainers::{
    core::{IntoContainerPort, Mount, ReuseDirective, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};
use tokio::time::Duration;

lazy_static! {

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

fn is_port_in_use(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_millis(1000),
    )
    .is_ok()
}

async fn mega_container(mapping_port: u16) -> ContainerAsync<GenericImage> {
    println!("MEGA {:?} ", MEGA.to_str().unwrap());
    println!("CONFIG {:?} ", CONFIG.to_str().unwrap());
    if !MEGA.exists() {
        panic!("mega binary not found in \"target/debug/\", skip lfs test");
    }
    if is_port_in_use(mapping_port) {
        panic!("port {mapping_port} is already in use");
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
        .with_wait_for(WaitFor::Http(Box::new(
            HttpWaitStrategy::new("/")
                .with_method(Method::GET)
                .with_expected_status_code(404_u16),
        )))
        .with_mapped_port(mapping_port, mapping_port.tcp())
        .with_mount(Mount::bind_mount(MEGA.to_str().unwrap(), "/root/mega"))
        .with_mount(Mount::bind_mount(
            CONFIG.to_str().unwrap(),
            "/root/config.toml",
        ))
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
    (container, format!("http://{mega_ip}:{mega_port}"))
}

#[tokio::test]
#[ignore]
///Use container to run mega server and test push and download
async fn test_push_object_and_download() {
    let (_container, mega_server_url) = mega_bootstrap_servers(12000).await;
    println!("container: {mega_server_url}");
    let file_map = mercury::test_utils::setup_lfs_file().await;
    let file = file_map
        .get("git-2d187177923cd618a75da6c6db45bb89d92bd504.pack")
        .unwrap();
    let client = LFSClient::from_url(&Url::parse(&mega_server_url).unwrap());
    let oid = lfs::calc_lfs_file_hash(file).unwrap();

    match client.push_object(&oid, file).await {
        Ok(_) => println!("Pushed successfully."),
        Err(err) => eprintln!("Push failed: {err:?}"),
    }
}
