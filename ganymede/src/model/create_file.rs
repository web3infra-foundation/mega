use crate::mega_node::MegaNode;
use common::utils::generate_id;
use callisto::mega_snapshot;
use std::cell::RefCell;

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct CreateFileInfo {
    /// can be a file or directory
    pub is_directory: bool,
    pub name: String,
    /// leave empty if it's under root
    pub path: String,
    pub import_dir: bool,
}



impl From<CreateFileInfo> for mega_snapshot::Model {
    fn from(value: CreateFileInfo) -> Self {
        mega_snapshot::Model {
            id: generate_id(),
            path: value.path,
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

impl From<CreateFileInfo> for MegaNode {
    fn from(value: CreateFileInfo) -> Self {
        MegaNode {
            name: value.name,
            path: value.path.parse().unwrap(),
            is_directory: value.is_directory,
            children: RefCell::new(vec![]),
            import_dir: value.import_dir,
        }
    }
}