//!
//!
//!
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use clap::Args;

use ed25519_dalek::{SigningKey, SIGNATURE_LENGTH};
use russh_keys::key::KeyPair;

use common::model::CommonOptions;
use storage::driver::database;

use crate::git_protocol::ssh::SshServer;

#[derive(Args, Clone, Debug)]
pub struct SshOptions {
    #[clap(flatten)]
    pub common: CommonOptions,

    #[clap(flatten)]
    pub custom: SshCustom,
}

#[derive(Args, Clone, Debug)]
pub struct SshCustom {
    #[arg(long, default_value_t = 8100)]
    ssh_port: u16,

    #[arg(long, value_name = "FILE")]
    ssh_key_path: Option<PathBuf>,

    #[arg(long, value_name = "FILE")]
    ssh_cert_path: Option<PathBuf>,
}

/// start a ssh server
pub async fn start_server(command: &SshOptions) {
    // we need to persist the key to prevent key expired after server restart.
    let client_key = load_key().await.unwrap();
    let client_pubkey = Arc::new(client_key.clone_public_key().unwrap());

    let mut config = russh::server::Config {
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        ..Default::default()
    };
    config.keys.push(client_key);

    let config = Arc::new(config);

    let SshOptions {
        common: CommonOptions { host, data_source },
        custom:
            SshCustom {
                ssh_port,
                ssh_key_path: _,
                ssh_cert_path: _,
            },
    } = command;
    let sh = SshServer {
        client_pubkey,
        clients: Arc::new(Mutex::new(HashMap::new())),
        id: 0,
        storage: database::init(data_source).await,
        pack_protocol: None,
        data_combined: Vec::new(),
    };
    let server_url = format!("{}:{}", host, ssh_port);
    let addr = SocketAddr::from_str(&server_url).unwrap();
    russh::server::run(config, addr, sh).await.unwrap()
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
    let key_root = env::var("SSH_ROOT").expect("SSH_ROOT is not set in .env file");
    let key_path = PathBuf::from(key_root).join("id_rsa");
    if !key_path.exists() {
        // generate a keypair if not exists
        let keys = KeyPair::generate_ed25519().unwrap();
        let mut key_file = File::create(&key_path).unwrap();

        let KeyPair::Ed25519(inner_pair) = keys;

        key_file.write_all(&inner_pair.to_keypair_bytes())?;

        Ok(KeyPair::Ed25519(inner_pair))
    } else {
        // load the keypair from the file
        let mut file = File::open(&key_path)?;
        let mut secret_key_bytes: [u8; SIGNATURE_LENGTH] = [0; SIGNATURE_LENGTH];
        file.read_exact(&mut secret_key_bytes)?;
        let keypair = SigningKey::from_keypair_bytes(&secret_key_bytes)?;
        tracing::info!("{:?}", keypair);
        Ok(KeyPair::Ed25519(keypair))
    }
}
