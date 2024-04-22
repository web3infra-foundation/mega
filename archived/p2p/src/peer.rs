//!
//!
//!
//!
//!

use clap::Args;
use std::str::FromStr;

use common::model::CommonOptions;
use secp256k1::{rand, SecretKey};

use crate::node::client;
use crate::node::relay_server;

/// Parameters for starting the p2p service
#[derive(Args, Clone, Debug)]
pub struct P2pCustom {
    #[arg(short, long, default_value_t = 8200)]
    pub p2p_port: u16,

    #[arg(long, default_value_t = 8001)]
    pub p2p_http_port: u16,

    #[arg(short, long, default_value_t = String::from(""))]
    pub bootstrap_node: String,

    #[arg(short, long)]
    pub secret_key: Option<String>,

    #[arg(short, long, default_value_t = false)]
    pub relay_server: bool,
}

#[derive(Args, Clone, Debug)]
pub struct P2pOptions {
    #[clap(flatten)]
    pub common: CommonOptions,

    #[clap(flatten)]
    pub custom: P2pCustom,
}

/// run as a p2p node
pub async fn run(options: &P2pOptions) -> Result<(), Box<dyn std::error::Error>> {
    let P2pOptions {
        common: CommonOptions { host, data_source },
        custom:
            P2pCustom {
                p2p_port,
                p2p_http_port,
                bootstrap_node,
                secret_key,
                relay_server,
            },
    } = options;
    let p2p_address = format!("/ip4/{}/tcp/{}", host, p2p_port);

    // Create a secret key.
    let secret_key = if let Some(secret_key) = secret_key {
        tracing::info!("Generate keys with fix {}", secret_key);
        SecretKey::from_str(secret_key.as_str()).unwrap()
    } else {
        tracing::info!("Generate keys randomly");
        SecretKey::new(&mut rand::thread_rng())
    };
    if *relay_server {
        relay_server::run(secret_key, p2p_address)?;
    } else {
        client::run(
            secret_key,
            p2p_address,
            bootstrap_node.clone(),
            *data_source,
            *p2p_http_port,
        )
        .await?;
    }
    Ok(())
}
