//!
//!
//!
//!
//!
//!

use database::driver::ObjectStorage;
use git::protocol::{PackProtocol, Protocol};
use std::path::PathBuf;
use std::sync::Arc;

pub mod network;
pub mod node;
pub mod peer;

async fn get_pack_protocol(path: &str, storage: Arc<dyn ObjectStorage>) -> PackProtocol {
    let path = del_ends_str(path, ".git");
    PackProtocol::new(PathBuf::from(path), storage, Protocol::P2p)
}

pub fn get_repo_full_path(repo_name: &str) -> String {
    let repo_name = del_ends_str(repo_name, ".git");
    "/root/".to_string() + repo_name
}

pub fn del_ends_str<'a>(mut s: &'a str, end: &str) -> &'a str {
    if s.ends_with(end) {
        s = s.split_at(s.len() - end.len()).0;
    }
    s
}

#[cfg(test)]
mod tests {}
