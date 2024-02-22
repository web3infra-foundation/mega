/// The `repo` struct maintains the relationship between `repo_id` and `repo_path`.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Repo {
    pub repo_id: i64,
    pub repo_path: String,
    pub repo_name: String,
}
