//!
//!
//!
//!
//!

use clap::Args;
use libp2p::identity;
use libp2p::identity::secp256k1::SecretKey;

use common::model::CommonOptions;

use crate::node::client;
use crate::node::relay_server;

/// Parameters for starting the p2p service
#[derive(Args, Clone, Debug)]
pub struct P2pCustom {
    #[arg(short, long, default_value_t = 8200)]
    pub p2p_port: u16,

    #[arg(short, long, default_value_t = String::from(""))]
    pub bootstrap_node: String,

    #[arg(short, long)]
    pub secret_key_seed: Option<u8>,

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
                bootstrap_node,
                secret_key_seed,
                relay_server,
            },
    } = options;
    let p2p_address = format!("/ip4/{}/tcp/{}", host, p2p_port);

    // Create a PeerId.
    let local_key = if let Some(secret_key_seed) = secret_key_seed {
        tracing::info!("Generate keys with fix seed={}", secret_key_seed);
        // generate_ed25519_fix(*secret_key_seed)
        generate_secp256k1_fix(*secret_key_seed)
    } else {
        tracing::info!("Generate keys randomly");
        // identity::Keypair::generate_ed25519()
        identity::Keypair::generate_secp256k1()
    };
    let sk = SecretKey::generate();
    if *relay_server {
        relay_server::run(local_key, p2p_address)?;
    } else {
        client::run(sk, p2p_address, bootstrap_node.clone(), *data_source).await?;
    }
    Ok(())
}

fn generate_secp256k1_fix(secret_key_seed: u8) -> identity::Keypair {
    let mut bytes = [0u8; 32];
    bytes[0] = secret_key_seed;

    identity::Keypair::secp256k1_from_der(&mut bytes).unwrap()
}
