use std::fmt;

use callisto::{ztm_node, ztm_repo_info};
use chrono::Utc;
use serde::{Deserialize, Serialize};

pub mod ca;
pub mod http;
pub mod ztm;

#[derive(Deserialize, Debug)]
pub struct RelayGetParams {
    pub peer_id: Option<String>,
    pub hub: Option<String>,
    pub name: Option<String>,
    pub agent_name: Option<String>,
    pub service_name: Option<String>,
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
