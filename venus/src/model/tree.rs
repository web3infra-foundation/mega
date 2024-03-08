use std::str::FromStr;

use callisto::db_enums::MergeStatus;
use callisto::mega_tree;
use callisto::mega_tree::Model;
use common::utils::generate_id;

use crate::internal::object::tree::Tree;
use crate::{hash::SHA1, internal::object::ObjectTrait};

impl From<Tree> for mega_tree::Model {
    fn from(value: Tree) -> Self {
        mega_tree::Model {
            id: generate_id(),
            tree_id: value.id.to_plain_str(),
            sub_trees: value.to_data().unwrap(),
            parent_id: None,
            name: String::new(),
            mr_id: String::new(),
            status: MergeStatus::Open,
            size: 0,
            full_path: String::new(),
            commit_id: String::new(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<mega_tree::Model> for Tree {
    fn from(value: Model) -> Self {
        Tree::from_bytes(value.sub_trees, SHA1::from_str(&value.tree_id).unwrap()).unwrap()
    }
}
