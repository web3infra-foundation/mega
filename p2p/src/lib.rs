//!
//!
//!
//!
//!
//!

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use git::protocol::{PackProtocol, Protocol};
use storage::driver::database::storage::ObjectStorage;

pub mod cbor;
pub mod internal;
pub mod network;
pub mod node;
pub mod nostr;
pub mod peer;

fn get_pack_protocol(path: &str, storage: Arc<dyn ObjectStorage>) -> PackProtocol {
    let path = del_ends_str(path, ".git");
    PackProtocol::new(PathBuf::from(path), storage, Protocol::P2p)
}

pub fn get_repo_full_path(repo_name: &str) -> String {
    let repo_name = del_ends_str(repo_name, ".git");
    "/projects/".to_string() + repo_name
}

pub fn del_ends_str<'a>(mut s: &'a str, end: &str) -> &'a str {
    if s.ends_with(end) {
        s = s.split_at(s.len() - end.len()).0;
    }
    s
}

pub fn get_utc_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {}
