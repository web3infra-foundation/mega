use callisto::git_repo;

use crate::internal::repo::Repo;

impl From<Repo> for git_repo::Model {
    fn from(value: Repo) -> Self {
        git_repo::Model {
            id: value.repo_id,
            repo_path: value.repo_path,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}
