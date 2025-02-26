use std::fmt;

use callisto::{lfs_objects, lfs_split_relations, ztm_lfs_info, ztm_node, ztm_repo_info};
use chrono::Utc;
use common::utils::generate_id;
use serde::{Deserialize, Serialize};
use util::get_utc_timestamp;

pub mod ca;
pub mod cache;
pub mod http;
pub mod lfs;
pub mod nostr;
pub mod p2p;
pub mod util;
pub mod ztm;

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
    pub hub: String,
    pub agent_name: String,
    pub service_name: String,
    pub mega_type: String,
    pub online: bool,
    pub last_online_time: i64,
    pub service_port: i32,
}

#[derive(Debug)]
pub enum ConversionError {
    InvalidParas,
}

impl TryFrom<RelayGetParams> for Node {
    type Error = ConversionError;

    fn try_from(paras: RelayGetParams) -> Result<Self, Self::Error> {
        if paras.peer_id.is_none()
            || paras.hub.is_none()
            || paras.agent_name.is_none()
            || paras.service_name.is_none()
            || paras.service_port.is_none()
        {
            return Err(ConversionError::InvalidParas);
        }
        let now = Utc::now().timestamp_millis();
        Ok(Node {
            peer_id: paras.peer_id.unwrap(),
            hub: paras.hub.unwrap(),
            agent_name: paras.agent_name.unwrap(),
            service_name: paras.service_name.unwrap(),
            mega_type: MegaType::Agent.to_string(),
            online: true,
            last_online_time: now,
            service_port: paras.service_port.unwrap(),
        })
    }
}

impl TryFrom<RelayGetParams> for ztm_node::Model {
    type Error = ConversionError;

    fn try_from(paras: RelayGetParams) -> Result<Self, Self::Error> {
        if paras.peer_id.is_none()
            || paras.hub.is_none()
            || paras.agent_name.is_none()
            || paras.service_name.is_none()
            || paras.service_port.is_none()
        {
            return Err(ConversionError::InvalidParas);
        }
        let now = Utc::now().timestamp_millis();
        Ok(ztm_node::Model {
            peer_id: paras.peer_id.unwrap(),
            hub: paras.hub.unwrap(),
            agent_name: paras.agent_name.unwrap(),
            service_name: paras.service_name.unwrap(),
            r#type: MegaType::Agent.to_string(),
            online: true,
            last_online_time: now,
            service_port: paras.service_port.unwrap(),
        })
    }
}

impl TryFrom<Node> for ztm_node::Model {
    type Error = ConversionError;

    fn try_from(n: Node) -> Result<Self, Self::Error> {
        Ok(ztm_node::Model {
            peer_id: n.peer_id,
            hub: n.hub,
            agent_name: n.agent_name,
            service_name: n.service_name,
            r#type: n.mega_type,
            online: n.online,
            last_online_time: n.last_online_time,
            service_port: n.service_port,
        })
    }
}

impl From<ztm_node::Model> for Node {
    fn from(n: ztm_node::Model) -> Self {
        Node {
            peer_id: n.peer_id,
            hub: n.hub,
            agent_name: n.agent_name,
            service_name: n.service_name,
            mega_type: n.r#type,
            online: n.online,
            last_online_time: n.last_online_time,
            service_port: n.service_port,
        }
    }
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

impl From<RepoInfo> for ztm_repo_info::Model {
    fn from(r: RepoInfo) -> Self {
        ztm_repo_info::Model {
            identifier: r.identifier,
            name: r.name,
            origin: r.origin,
            update_time: r.update_time,
            commit: r.commit,
        }
    }
}

impl From<ztm_repo_info::Model> for RepoInfo {
    fn from(r: ztm_repo_info::Model) -> Self {
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

impl From<LFSInfo> for ztm_lfs_info::Model {
    fn from(r: LFSInfo) -> Self {
        ztm_lfs_info::Model {
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

impl From<ztm_lfs_info::Model> for LFSInfo {
    fn from(r: ztm_lfs_info::Model) -> Self {
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

impl From<LFSInfoPostBody> for ztm_lfs_info::Model {
    fn from(r: LFSInfoPostBody) -> Self {
        ztm_lfs_info::Model {
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
