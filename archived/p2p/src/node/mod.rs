//!
//!
//!
//!
//!
//!

use libp2p::identity::Keypair;
use libp2p::identity::{self, secp256k1::SecretKey};
use serde::{Deserialize, Serialize};

pub mod client;
pub mod relay_server;

#[cfg(test)]
mod tests {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct MegaRepoInfo {
    pub origin: String,
    pub name: String,
    pub latest: String,
    pub forks: Vec<Fork>,
    pub timestamp: i64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Fork {
    pub peer: String,
    pub latest: String,
    pub timestamp: i64,
}

pub fn sk_to_local_key(secret_key: secp256k1::SecretKey) -> Keypair {
    let sk = SecretKey::try_from_bytes(secret_key.secret_bytes()).unwrap();
    let secp256k1_kp = identity::secp256k1::Keypair::from(sk);
    identity::Keypair::from(secp256k1_kp)
}
