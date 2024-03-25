use std::str::FromStr;

use callisto::{db_enums::MergeStatus, mega_commit};
use common::utils::generate_id;

use crate::{
    hash::SHA1,
    internal::object::{commit::Commit, signature::Signature},
};

impl From<mega_commit::Model> for Commit {
    fn from(value: mega_commit::Model) -> Self {
        Commit {
            id: SHA1::from_str(&value.commit_id).unwrap(),
            tree_id: SHA1::from_str(&value.tree).unwrap(),
            parent_commit_ids: value
                .parents_id
                .into_iter()
                .map(|id| SHA1::from_str(&id).unwrap())
                .collect(),
            author: Signature::from_data(value.author.unwrap().into()).unwrap(),
            committer: Signature::from_data(value.committer.unwrap().into()).unwrap(),
            message: value.content.unwrap(),
        }
    }
}

impl From<Commit> for mega_commit::Model {
    fn from(value: Commit) -> Self {
        mega_commit::Model {
            id: generate_id(),
            repo_id: 0,
            commit_id: value.id.to_plain_str(),
            tree: value.tree_id.to_plain_str(),
            parents_id: value
                .parent_commit_ids
                .iter()
                .map(|x| x.to_plain_str())
                .collect(),
            author: Some(String::from_utf8_lossy(&value.author.to_data().unwrap()).to_string()),
            committer: Some(String::from_utf8_lossy(&value.committer.to_data().unwrap()).to_string()),
            content: Some(value.message.clone()),
            mr_id: 0,
            status: MergeStatus::Open,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}
