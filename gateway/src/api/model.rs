use serde::Deserialize;

use callisto::relay_path_mapping;
use common::utils::generate_id;

#[derive(Debug, Deserialize, Clone)]
pub struct RepoProvideQuery {
    pub alias: String,
    pub path: String,
}

impl From<RepoProvideQuery> for relay_path_mapping::Model {
    fn from(value: RepoProvideQuery) -> Self {
        Self {
            id: generate_id(),
            alias: value.alias,
            repo_path: value.path,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}
