use serde::{Deserialize, Serialize};

pub mod crate_repo;
pub mod crates;
pub mod crates_pro_message;
pub mod github_repo;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataSource {
    Cratesio,
    Github,
}
