use serde::{Deserialize, Serialize};

pub mod crate_repo;
pub mod crates;
pub mod github_repo;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataSource {
    Freighter,
    Manual,
}
