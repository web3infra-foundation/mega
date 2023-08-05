//!
//!
//!
//!
//!
//!

use libp2p::rendezvous::Cookie;
use libp2p::request_response::RequestId;
use libp2p::{Multiaddr, PeerId};
use std::collections::HashMap;

pub mod client;
mod input_command;
pub mod relay_server;

#[cfg(test)]
mod tests {}

pub struct MegaRepoInfo {
    pub upstream: String,
    pub peer_id: String,
    pub object_id: String,
    pub timestamp: i64,
}

pub struct ClientParas {
    pub cookie: Option<Cookie>,
    pub rendezvous_point: Option<PeerId>,
    pub bootstrap_node_addr: Option<Multiaddr>,
    pub pending_git_upload_package: HashMap<RequestId, String>,
}

pub fn get_repo_full_path(repo_name: &str) -> String {
    "/root/".to_string() + repo_name
}
