use std::str::FromStr;

use callisto::{git_tree, mega_tree};

use crate::internal::model::generate_id;
use crate::internal::object::tree::Tree;
use crate::{hash::SHA1, internal::object::ObjectTrait};

impl From<Tree> for mega_tree::Model {
    fn from(value: Tree) -> Self {
        mega_tree::Model {
            id: generate_id(),
            tree_id: value.id.to_string(),
            sub_trees: value.to_data().unwrap(),
            size: 0,
            commit_id: String::new(),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<Tree> for git_tree::Model {
    fn from(value: Tree) -> Self {
        git_tree::Model {
            id: generate_id(),
            repo_id: 0,
            tree_id: value.id.to_string(),
            sub_trees: value.to_data().unwrap(),
            size: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<mega_tree::Model> for Tree {
    fn from(value: mega_tree::Model) -> Self {
        Tree::from_bytes(&value.sub_trees, SHA1::from_str(&value.tree_id).unwrap()).unwrap()
    }
}

impl From<git_tree::Model> for Tree {
    fn from(value: git_tree::Model) -> Self {
        Tree::from_bytes(&value.sub_trees, SHA1::from_str(&value.tree_id).unwrap()).unwrap()
    }
}
