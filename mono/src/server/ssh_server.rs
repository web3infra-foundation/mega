use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use bytes::BytesMut;
use clap::Args;

use common::config::Config;
use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use ed25519_dalek::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use ed25519_dalek::SigningKey;
use russh::server::Server;
use russh_keys::key::KeyPair;

use common::model::CommonOptions;
use jupiter::context::Context;
use tokio::sync::Mutex;
use vault::vault::{read_secret, write_secret};

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
    #[arg(long, default_value_t = 2222)]
    ssh_port: u16,
}

/// start a ssh server
pub async fn start_server(config: Config, command: &SshOptions) {
    // we need to persist the key to prevent key expired after server restart.
    let client_key = load_key();
    let client_pubkey = Arc::new(client_key.clone_public_key().unwrap());

    let mut ru_config = russh::server::Config {
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        // preferred: Preferred {
        //     key: &[russh_keys::key::SSH_RSA],
        //     ..Default::default()
        // },
        ..Default::default()
    };
    ru_config.keys.push(client_key);

    let ru_config = Arc::new(ru_config);

    let SshOptions {
        common: CommonOptions { host, .. },
        custom: SshCustom { ssh_port },
    } = command;
    let context = Context::new(config.clone()).await;
    context.services.mono_storage.init_monorepo(&config.monorepo).await;
    let mut ssh_server = SshServer {
        client_pubkey,
        clients: Arc::new(Mutex::new(HashMap::new())),
        id: 0,
        context,
        smart_protocol: None,
        data_combined: BytesMut::new(),
    };
    let server_url = format!("{}:{}", host, ssh_port);
    let addr = SocketAddr::from_str(&server_url).unwrap();
    ssh_server.run_on_address(ru_config, addr).await.unwrap();
}

pub fn load_key() -> KeyPair {
    let ssh_key = read_secret("ssh_server_key").unwrap();
    if let Some(ssh_key) = ssh_key {
        // load the keypair from the vault
        let data = ssh_key.data.unwrap();
        let secret_key = data["secret_key"].as_str().unwrap().to_string();
        let keypair = SigningKey::from_pkcs8_pem(&secret_key).expect("parsing key err");
        KeyPair::Ed25519(keypair)
    } else {
        // generate a keypair if not exists
        let keys = KeyPair::generate_ed25519().unwrap();
        if let KeyPair::Ed25519(inner_pair) = &keys {
            let secret = serde_json::json!({
                "secret_key": *inner_pair.to_pkcs8_pem(LineEnding::CR).unwrap()
            })
            .as_object()
            .unwrap()
            .clone();
            write_secret("ssh_server_key", Some(secret)).unwrap_or_else(|e| {
                panic!("Failed to write ssh_server_key: {:?}", e);
            });
        }
        keys
    }
}
