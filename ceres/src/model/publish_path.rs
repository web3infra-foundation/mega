use common::utils::generate_id;
use serde::{Deserialize, Serialize};

use crate::protocol::repo::Repo;

#[derive(PartialEq, Eq, Debug, Clone, Default, Serialize, Deserialize)]
pub struct PublishPathInfo {
    pub repo_name: String,
    pub path: String,
}

impl From<PublishPathInfo> for Repo {
    fn from(value: PublishPathInfo) -> Self {
        Self {
            repo_id: generate_id(),
            repo_path: value.path,
            repo_name: value.repo_name,
            is_monorepo: true,
        }
    }
}
