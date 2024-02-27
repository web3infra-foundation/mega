use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use common::utils::generate_id;
use db_entity::db_enums::MergeStatus;
use db_entity::{mega_snapshot, mega_tree};

use crate::internal::object::tree::Tree;

#[derive(Debug, Clone, PartialEq)]
pub struct MegaNode {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub children: RefCell<Vec<Rc<MegaNode>>>,
    pub import_dir: bool,
}

impl MegaNode {
    pub fn add_child(&self, node: &Rc<MegaNode>) {
        self.children.borrow_mut().push(Rc::clone(node))
    }

    pub fn convert_to_mega_tree(&self, tree: &Tree) {
        let mut model: mega_tree::Model = self.to_owned().into();
        model.tree_id = tree.id.to_plain_str();
        model.sub_trees = tree.tree_items.iter().map(|x| x.to_string()).collect();
        model.size = 0;
    }
}

// Entity <==> Node <==> Git Objects
impl From<MegaNode> for mega_snapshot::Model {
    fn from(value: MegaNode) -> Self {
        mega_snapshot::Model {
            id: generate_id(),
            path: value.path.to_str().unwrap().to_owned(),
            name: value.name,
            import_dir: value.import_dir,
            tree_id: None,
            sub_trees: None,
            commit_id: None,
            size: 0,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<MegaNode> for mega_tree::Model {
    fn from(value: MegaNode) -> Self {
        mega_tree::Model {
            id: generate_id(),
            full_path: value.path.to_str().unwrap().to_owned(),
            import_dir: value.import_dir,
            tree_id: String::new(),
            sub_trees: Vec::new(),
            size: 0,
            mr_id: String::new(),
            status: MergeStatus::Merged,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}


