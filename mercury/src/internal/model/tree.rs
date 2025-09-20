use std::str::FromStr;

use crate::internal::model::sea_models::{git_tree as sea_git_tree, mega_tree as sea_mega_tree};

use crate::internal::model::generate_id;
use crate::internal::object::tree::Tree;
use crate::{hash::SHA1, internal::object::ObjectTrait};

impl From<Tree> for sea_mega_tree::Model {
    fn from(value: Tree) -> Self {
        sea_mega_tree::Model {
            id: generate_id(),
            tree_id: value.id.to_string(),
            sub_trees: value.to_data().unwrap(),
            size: 0,
            commit_id: String::new(),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<Tree> for sea_git_tree::Model {
    fn from(value: Tree) -> Self {
        sea_git_tree::Model {
            id: generate_id(),
            repo_id: 0,
            tree_id: value.id.to_string(),
            sub_trees: value.to_data().unwrap(),
            size: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<sea_mega_tree::Model> for Tree {
    fn from(value: sea_mega_tree::Model) -> Self {
        Tree::from_bytes(&value.sub_trees, SHA1::from_str(&value.tree_id).unwrap()).unwrap()
    }
}

impl From<sea_git_tree::Model> for Tree {
    fn from(value: sea_git_tree::Model) -> Self {
        Tree::from_bytes(&value.sub_trees, SHA1::from_str(&value.tree_id).unwrap()).unwrap()
    }
}
