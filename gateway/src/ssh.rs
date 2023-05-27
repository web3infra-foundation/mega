//!
//!
//!
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use anyhow::Result;

use clap::Args;
use russh_keys::key::KeyPair;

use storage::driver::mysql;
use tokio::io::AsyncWriteExt;

use git::protocol::ssh::SshServer;

#[derive(Args, Clone, Debug)]
pub struct SshOptions {
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    host: String,

    #[arg(short, long, default_value_t = 8000)]
    port: u16,

    #[arg(short, long, value_name = "FILE")]
    key_path: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    cert_path: Option<PathBuf>,

    #[arg(short, long, default_value_os_t = PathBuf::from("lfs_content"))]
    lfs_content_path: PathBuf,
}

/// start a ssh server
pub async fn server(command: &SshOptions) -> Result<(), std::io::Error> {
    let client_key = load_key().await.unwrap();
    let client_pubkey = Arc::new(client_key.clone_public_key().unwrap());

    let mut config = russh::server::Config {
        connection_timeout: Some(std::time::Duration::from_secs(10)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        ..Default::default()
    };
    config.keys.push(client_key);

    let config = Arc::new(config);
    let sh = SshServer {
        client_pubkey,
        clients: Arc::new(Mutex::new(HashMap::new())),
        id: 0,
        storage: Arc::new(mysql::init().await),
        pack_protocol: None,
    };

    let SshOptions {
        host,
        port,
        key_path: _,
        cert_path: _,
        lfs_content_path: _,
    } = command;
    let server_url = format!("{}:{}", host, port);
    let addr = SocketAddr::from_str(&server_url).unwrap();
    russh::server::run(config, addr, sh).await
}

/// # Loads an SSH keypair.
///
/// This function follows the following steps:
/// 1. It retrieves the root directory for the SSH key from the environment variable SSH_ROOT using env::var.
/// 2. It constructs the path to the SSH private key file by joining the root directory with the filename "id_rsa" using PathBuf.
/// 3. It checks if the key file exists. If it doesn't, it generates a new Ed25519 keypair using KeyPair::generate_ed25519.
/// - The generated keypair is then written to the key file.
/// 4. If the key file exists, it reads the keypair from the file.
/// - The keypair is loaded from the file and returned.
///
/// # Returns
///
/// An asynchronous Result containing the loaded SSH keypair if successful, or an error if any of the steps fail.
async fn load_key() -> Result<KeyPair> {
    let key_root = env::var("SSH_ROOT").expect("WORK_DIR is not set in .env file");
    let key_path = PathBuf::from(key_root).join("id_rsa");
    if !key_path.exists() {
        // generate a keypair if not exists
        let keys = KeyPair::generate_ed25519().unwrap();
        let mut key_file = tokio::fs::File::create(&key_path).await.unwrap();

        let KeyPair::Ed25519(inner_pair) = keys;

        key_file.write_all(&inner_pair.to_bytes()).await?;

        Ok(KeyPair::Ed25519(inner_pair))
    } else {
        // load the keypair from the file
        let key_data = tokio::fs::read(&key_path).await?;
        let keypair = ed25519_dalek::Keypair::from_bytes(&key_data)?;

        Ok(KeyPair::Ed25519(keypair))
    }
}
