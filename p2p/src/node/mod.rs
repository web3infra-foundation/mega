//!
//!
//!
//!
//!
//!

use std::collections::HashMap;
use std::sync::Arc;

use libp2p::identity::secp256k1::SecretKey;
use libp2p::identity::Keypair;
use libp2p::kad::QueryId;
use libp2p::rendezvous::Cookie;
use libp2p::request_response::OutboundRequestId;
use libp2p::{identity, Multiaddr, PeerId};
use secp256k1::KeyPair;
use serde::{Deserialize, Serialize};

use entity::objects::Model;
use storage::driver::database::storage::ObjectStorage;

pub mod client;
mod client_http;
mod command_handler;
mod input_command;
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

pub struct ClientParas {
    pub cookie: Option<Cookie>,
    pub rendezvous_point: Option<PeerId>,
    pub bootstrap_node_addr: Option<Multiaddr>,
    pub storage: Arc<dyn ObjectStorage>,
    pub key_pair: KeyPair,
    pub pending_git_upload_package: HashMap<OutboundRequestId, String>,
    pub pending_git_pull: HashMap<OutboundRequestId, String>,
    pub pending_git_obj_download: HashMap<OutboundRequestId, String>,
    pub pending_repo_info_update_fork: HashMap<QueryId, String>,
    pub pending_repo_info_search_to_download_obj: HashMap<QueryId, String>,
    pub pending_git_obj_id_download: HashMap<OutboundRequestId, String>,
    pub repo_node_list: HashMap<String, Vec<String>>,
    pub repo_id_need_list: HashMap<String, Vec<String>>,
    // pub repo_receive_git_obj_model_list: HashMap<String, Vec<Model>>,
    pub repo_receive_git_obj_model_list: HashMap<String, Vec<Model>>,
}

pub fn sk_to_local_key(secret_key: secp256k1::SecretKey) -> Keypair {
    let sk = SecretKey::try_from_bytes(secret_key.secret_bytes()).unwrap();
    let secp256k1_kp = identity::secp256k1::Keypair::from(sk);
    identity::Keypair::from(secp256k1_kp)
}
