//!
//!
//!
//!
//!
//!

use database::driver::ObjectStorage;
use entity::git_obj::Model;
use libp2p::kad::QueryId;
use libp2p::rendezvous::Cookie;
use libp2p::request_response::RequestId;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod client;
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
    pub pending_git_upload_package: HashMap<RequestId, String>,
    pub pending_git_pull: HashMap<RequestId, String>,
    pub pending_git_obj_download: HashMap<RequestId, String>,
    pub pending_repo_info_update_fork: HashMap<QueryId, String>,
    pub pending_repo_info_search_to_download_obj: HashMap<QueryId, String>,
    pub pending_git_obj_id_download: HashMap<RequestId, String>,
    pub repo_node_list: HashMap<String, Vec<String>>,
    pub repo_id_need_list: Arc<Mutex<HashMap<String, Vec<String>>>>,
    // pub repo_receive_git_obj_model_list: HashMap<String, Vec<Model>>,
    pub repo_receive_git_obj_model_list: Arc<Mutex<HashMap<String, Vec<Model>>>>,
}

pub fn get_utc_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}
