use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::crates::CrateMessage;
use crate::model::DataSource;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateRepoMessage {
    pub crate_name: String,
    pub crate_version: String,
    pub cksum: String,
    pub data_source: DataSource,
    pub clone_url: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uuid: String,
}

impl From<CrateMessage> for CrateRepoMessage {
    fn from(value: CrateMessage) -> Self {
        Self {
            crate_name: value.crate_name,
            crate_version: value.crate_version,
            cksum: value.cksum,
            data_source: value.data_source,
            clone_url: String::new(),
            timestamp: Utc::now(),
            version: value.version,
            uuid: String::new(),
        }
    }
}
