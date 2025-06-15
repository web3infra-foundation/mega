use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use bytes::BytesMut;
use clap::Args;
use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use russh::{
    keys::{ssh_key::rand_core::OsRng, Algorithm, PrivateKey},
    server::Server,
    Preferred,
};

use common::model::CommonHttpOptions;
use jupiter::context::Context;
use tokio::sync::Mutex;
use vault::vault::{read_secret, write_secret};

use crate::git_protocol::ssh::SshServer;

#[derive(Args, Clone, Debug)]
pub struct SshOptions {
    #[clap(flatten)]
    pub common: CommonHttpOptions,

    #[clap(flatten)]
    pub custom: SshCustom,
}

#[derive(Args, Clone, Debug)]
pub struct SshCustom {
    #[arg(long, default_value_t = 2222)]
    ssh_port: u16,
}

/// start an ssh server
pub async fn start_server(context: Context, command: &SshOptions) {
    // we need to persist the key to prevent key expired after server restart.
    let p_key = load_key().await;
    let ru_config = russh::server::Config {
        auth_rejection_time: std::time::Duration::from_secs(3),
        keys: vec![p_key],
        preferred: Preferred {
            // key: Cow::Borrowed(&[CERT_ECDSA_SHA2_P256]),
            ..Preferred::default()
        },
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        ..Default::default()
    };

    let ru_config = Arc::new(ru_config);

    let SshOptions {
        common: CommonHttpOptions { host, .. },
        custom: SshCustom { ssh_port },
    } = command;
    let mut ssh_server = SshServer {
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

pub async fn load_key() -> PrivateKey {
    let ssh_key = read_secret("ssh_server_key").await.unwrap();
    if let Some(ssh_key) = ssh_key {
        let data = ssh_key.data.unwrap();
        let secret_key = data["secret_key"].as_str().unwrap();
        PrivateKey::from_openssh(secret_key).unwrap()
    } else {
        // generate a keypair if not exists
        let keys = PrivateKey::random(&mut OsRng, Algorithm::Ed25519).unwrap();
        let secret = serde_json::json!({
            "secret_key":
            *keys.to_openssh(LineEnding::CR).unwrap()
        })
        .as_object()
        .unwrap()
        .clone();
        write_secret("ssh_server_key", Some(secret))
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to write ssh_server_key: {:?}", e);
            });
        keys
    }
}
