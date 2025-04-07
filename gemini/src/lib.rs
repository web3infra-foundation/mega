use std::fmt;

use callisto::{git_repo, lfs_objects, lfs_split_relations, relay_lfs_info, relay_repo_info};
use common::utils::generate_id;
use serde::{Deserialize, Serialize};
use util::get_utc_timestamp;

pub mod ca;
pub mod lfs;
pub mod nostr;
pub mod p2p;
pub mod util;

#[derive(Deserialize, Debug)]
pub struct RelayGetParams {
    pub peer_id: Option<String>,
    pub hub: Option<String>,
    pub name: Option<String>,
    pub agent_name: Option<String>,
    pub service_name: Option<String>,
    pub service_port: Option<i32>,
    pub file_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RelayResultRes {
    pub success: bool,
}

#[derive(Debug)]
pub enum MegaType {
    Agent,
    Relay,
}

impl fmt::Display for MegaType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MegaType::Agent => write!(f, "Agent"),
            MegaType::Relay => write!(f, "Relay"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub peer_id: String,
    pub service_name: String,
    pub mega_type: String,
    pub online: bool,
    pub last_online_time: i64,
}

#[derive(Debug)]
pub enum ConversionError {
    InvalidParas,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RepoInfo {
    pub name: String,
    pub identifier: String,
    pub origin: String,
    pub update_time: i64,
    pub commit: String,
    pub peer_online: bool,
}

impl RepoInfo {
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

impl From<RepoInfo> for relay_repo_info::Model {
    fn from(r: RepoInfo) -> Self {
        relay_repo_info::Model {
            identifier: r.identifier,
            name: r.name,
            origin: r.origin,
            update_time: r.update_time,
            commit: r.commit,
        }
    }
}

impl From<relay_repo_info::Model> for RepoInfo {
    fn from(r: relay_repo_info::Model) -> Self {
        RepoInfo {
            identifier: r.identifier,
            name: r.name,
            origin: r.origin,
            update_time: r.update_time,
            commit: r.commit,
            peer_online: false,
        }
    }
}

impl From<git_repo::Model> for RepoInfo {
    fn from(r: git_repo::Model) -> Self {
        RepoInfo {
            identifier: "".to_string(),
            name: r.repo_name,
            origin: "".to_string(),
            update_time: r.updated_at.and_utc().timestamp_millis(),
            commit: "".to_string(),
            peer_online: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LFSInfo {
    pub file_hash: String,
    pub hash_type: String,
    pub file_size: i64,
    pub creation_time: i64,
    pub peer_id: String,
    pub origin: String,
    pub peer_online: bool,
}

impl From<LFSInfo> for relay_lfs_info::Model {
    fn from(r: LFSInfo) -> Self {
        relay_lfs_info::Model {
            id: generate_id(),
            file_hash: r.file_hash,
            hash_type: r.hash_type,
            file_size: r.file_size,
            creation_time: r.creation_time,
            peer_id: r.peer_id,
            origin: r.origin,
        }
    }
}

impl From<relay_lfs_info::Model> for LFSInfo {
    fn from(r: relay_lfs_info::Model) -> Self {
        LFSInfo {
            file_hash: r.file_hash,
            hash_type: r.hash_type,
            file_size: r.file_size,
            creation_time: r.creation_time,
            peer_id: r.peer_id,
            origin: r.origin,
            peer_online: false,
        }
    }
}

impl From<LFSInfo> for LFSInfoPostBody {
    fn from(r: LFSInfo) -> Self {
        LFSInfoPostBody {
            file_hash: r.file_hash,
            hash_type: r.hash_type,
            file_size: r.file_size,
            peer_id: r.peer_id,
            origin: r.origin,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LFSInfoPostBody {
    pub file_hash: String,
    pub hash_type: String,
    pub file_size: i64,
    pub peer_id: String,
    pub origin: String,
}

impl From<LFSInfoPostBody> for relay_lfs_info::Model {
    fn from(r: LFSInfoPostBody) -> Self {
        relay_lfs_info::Model {
            id: generate_id(),
            file_hash: r.file_hash,
            hash_type: r.hash_type,
            file_size: r.file_size,
            creation_time: get_utc_timestamp(),
            peer_id: r.peer_id,
            origin: r.origin,
        }
    }
}

impl From<LFSInfoPostBody> for LFSInfo {
    fn from(r: LFSInfoPostBody) -> Self {
        LFSInfo {
            file_hash: r.file_hash,
            hash_type: r.hash_type,
            file_size: r.file_size,
            creation_time: get_utc_timestamp(),
            peer_id: r.peer_id,
            origin: r.origin,
            peer_online: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LFSInfoRes {
    pub oid: String,
    pub size: i64,
    pub chunks: Vec<LFSChunk>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LFSChunk {
    pub sub_oid: String,
    pub offset: i64,
    pub size: i64,
}

impl From<lfs_objects::Model> for LFSInfoRes {
    fn from(lfs: lfs_objects::Model) -> Self {
        LFSInfoRes {
            oid: lfs.oid,
            size: lfs.size,
            chunks: vec![],
        }
    }
}

impl From<lfs_split_relations::Model> for LFSChunk {
    fn from(chunk: lfs_split_relations::Model) -> Self {
        LFSChunk {
            sub_oid: chunk.sub_oid,
            offset: chunk.offset,
            size: chunk.size,
        }
    }
}
