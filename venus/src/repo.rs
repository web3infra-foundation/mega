use callisto::git_repo;

/// The `repo` struct maintains the relationship between `repo_id` and `repo_path`.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Repo {
    pub repo_id: i64,
    pub repo_path: String,
    pub repo_name: String,
}

impl Repo {
    pub fn empty() -> Self {
        Self {
            repo_id: 0,
            repo_path: String::new(),
            repo_name: String::new(),
        }
    }
}

impl From<git_repo::Model> for Repo {
    fn from(value: git_repo::Model) -> Self {
        Self {
            repo_id: value.id,
            repo_path: value.repo_path,
            repo_name: value.repo_name,
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
