use std::str::FromStr;

use callisto::{git_commit, mega_commit};
use common::utils::generate_id;

use crate::{
    hash::SHA1,
    internal::{
        object::{commit::Commit, signature::Signature, ObjectTrait},
        pack::entry::Entry,
    },
};

impl From<mega_commit::Model> for Commit {
    fn from(value: mega_commit::Model) -> Self {
        Commit {
            id: SHA1::from_str(&value.commit_id).unwrap(),
            tree_id: SHA1::from_str(&value.tree).unwrap(),
            parent_commit_ids: value
                .parents_id
                .as_array()
                .unwrap()
                .iter()
                .map(|id| SHA1::from_str(id.as_str().unwrap()).unwrap())
                .collect(),
            author: Signature::from_data(value.author.unwrap().into()).unwrap(),
            committer: Signature::from_data(value.committer.unwrap().into()).unwrap(),
            message: value.content.unwrap(),
        }
    }
}

impl From<git_commit::Model> for Commit {
    fn from(value: git_commit::Model) -> Self {
        Commit {
            id: SHA1::from_str(&value.commit_id).unwrap(),
            tree_id: SHA1::from_str(&value.tree).unwrap(),
            parent_commit_ids: value
                .parents_id
                .as_array()
                .unwrap()
                .iter()
                .map(|id| SHA1::from_str(id.as_str().unwrap()).unwrap())
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
            commit_id: value.id.to_string(),
            tree: value.tree_id.to_string(),
            parents_id: value
                .parent_commit_ids
                .iter()
                .map(|x| x.to_string())
                .collect(),
            author: Some(String::from_utf8_lossy(&value.author.to_data().unwrap()).to_string()),
            committer: Some(
                String::from_utf8_lossy(&value.committer.to_data().unwrap()).to_string(),
            ),
            content: Some(value.message.clone()),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<Commit> for git_commit::Model {
    fn from(value: Commit) -> Self {
        git_commit::Model {
            id: generate_id(),
            repo_id: 0,
            commit_id: value.id.to_string(),
            tree: value.tree_id.to_string(),
            parents_id: value
                .parent_commit_ids
                .iter()
                .map(|x| x.to_string())
                .collect(),
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
