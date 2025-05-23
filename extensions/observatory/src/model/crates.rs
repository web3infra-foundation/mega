use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::DataSource;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateMessage {
    pub crate_name: String,
    pub crate_version: String,
    pub cksum: String,
    pub data_source: DataSource,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uuid: String,
}
