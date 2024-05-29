use serde::{Deserialize, Serialize};

pub mod handler;

#[derive(Deserialize, Serialize, Debug)]
pub struct ZTMUserPermit {
    pub ca: String,
    pub agent: Agent,
    pub bootstraps: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Agent {
    pub certificate: String,
    pub private_key: String,
}
