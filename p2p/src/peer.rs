//!
//!
//!
//!
//!

use super::node::client;
use super::node::relay_server;
use clap::Args;
use database::DataSource;
use libp2p::identity;

/// Parameters for starting the p2p service
#[derive(Args, Clone, Debug)]
pub struct P2pOptions {
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub host: String,

    #[arg(short, long, default_value_t = 8001)]
    pub port: u16,

    #[arg(short, long, default_value_t = String::from(""))]
    pub bootstrap_node: String,

    #[arg(short, long)]
    pub secret_key_seed: Option<u8>,

    #[arg(short, long, default_value_t = false)]
    pub relay_server: bool,

    #[arg(value_enum, default_value = "mysql")]
    pub data_source: DataSource,
}

/// run as a p2p node
pub async fn run(options: &P2pOptions) -> Result<(), Box<dyn std::error::Error>> {
    let P2pOptions {
        host,
        port,
        bootstrap_node,
        secret_key_seed,
        relay_server,
        data_source,
    } = options;
    let p2p_address = format!("/ip4/{}/tcp/{}", host, port);

    // Create a PeerId.
    let local_key = if let Some(secret_key_seed) = secret_key_seed {
        tracing::info!("Generate keys with fix seed={}", secret_key_seed);
        generate_ed25519_fix(*secret_key_seed)
    } else {
        tracing::info!("Generate keys randomly");
        identity::Keypair::generate_ed25519()
    };

    if *relay_server {
        relay_server::run(local_key, p2p_address)?;
    } else {
        client::run(local_key, p2p_address, bootstrap_node.clone(), *data_source).await?;
    }
    Ok(())
}

fn generate_ed25519_fix(secret_key_seed: u8) -> identity::Keypair {
    let mut bytes = [0u8; 32];
    bytes[0] = secret_key_seed;

    identity::Keypair::ed25519_from_bytes(bytes).expect("only errors on wrong length")
}
