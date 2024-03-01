use callisto::db_enums::MergeStatus;
use callisto::mega_tree;
use common::utils::generate_id;

use crate::internal::object::tree::Tree;

impl From<Tree> for mega_tree::Model {
    fn from(value: Tree) -> Self {
        mega_tree::Model {
            id: generate_id(),
            tree_id: value.id.to_plain_str(),
            sub_trees: value.tree_items.iter().map(|x| x.to_string()).collect(),
            name: String::new(),
            mr_id: String::new(),
            status: MergeStatus::Merged,
            size: 0,
            full_path: String::new(),
            commit_id: String::new(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}
