use std::path::PathBuf;

use callisto::git_repo;
use common::utils::generate_id;

/// The `repo` struct maintains the relationship between `repo_id` and `repo_path`.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Repo {
    pub repo_id: i64,
    pub repo_path: String,
    pub repo_name: String,
    pub is_monorepo: bool,
}

impl Repo {
    pub fn new(path: PathBuf, is_monorepo: bool) -> Self {
        Self {
            repo_id: generate_id(),
            repo_path: path.to_str().unwrap().to_owned(),
            repo_name: path.file_name().unwrap().to_str().unwrap().to_owned(),
            is_monorepo,
        }
    }
}

impl From<git_repo::Model> for Repo {
    fn from(value: git_repo::Model) -> Self {
        Self {
            repo_id: value.id,
            repo_path: value.repo_path,
            repo_name: value.repo_name,
            is_monorepo: false,
        }
    }
}

impl From<Repo> for git_repo::Model {
    fn from(value: Repo) -> Self {
        git_repo::Model {
            id: value.repo_id,
            repo_path: value.repo_path,
            repo_name: value.repo_name,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}
