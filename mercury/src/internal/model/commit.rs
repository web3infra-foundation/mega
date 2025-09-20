use crate::internal::model::generate_id;
use crate::internal::model::sea_models::{
    git_commit as sea_git_commit, mega_commit as sea_mega_commit,
};
use crate::internal::{
    object::{commit::Commit, ObjectTrait},
    pack::entry::Entry,
};
use serde_json;

impl From<Commit> for sea_mega_commit::Model {
    fn from(value: Commit) -> Self {
        sea_mega_commit::Model {
            id: generate_id(),
            commit_id: value.id.to_string(),
            tree: value.tree_id.to_string(),
            parents_id: serde_json::to_string(
                &value
                    .parent_commit_ids
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>(),
            )
            .unwrap(),
            author: Some(String::from_utf8_lossy(&value.author.to_data().unwrap()).to_string()),
            committer: Some(
                String::from_utf8_lossy(&value.committer.to_data().unwrap()).to_string(),
            ),
            content: Some(value.message.clone()),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<Commit> for sea_git_commit::Model {
    fn from(value: Commit) -> Self {
        sea_git_commit::Model {
            id: generate_id(),
            repo_id: 0,
            commit_id: value.id.to_string(),
            tree: value.tree_id.to_string(),
            parents_id: serde_json::to_string(
                &value
                    .parent_commit_ids
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>(),
            )
            .unwrap(),
            author: Some(String::from_utf8_lossy(&value.author.to_data().unwrap()).to_string()),
            committer: Some(
                String::from_utf8_lossy(&value.committer.to_data().unwrap()).to_string(),
            ),
            content: Some(value.message.clone()),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<Entry> for Commit {
    fn from(value: Entry) -> Self {
        Commit::from_bytes(&value.data, value.hash).unwrap()
    }
}
