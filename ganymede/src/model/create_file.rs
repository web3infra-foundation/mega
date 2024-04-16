use std::cell::RefCell;

use serde::{Deserialize, Serialize};

use venus::{hash::SHA1, internal::object::tree::TreeItemMode};

use crate::mega_node::MegaNode;

#[derive(PartialEq, Eq, Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateFileInfo {
    /// can be a file or directory
    pub is_directory: bool,
    pub name: String,
    /// leave empty if it's under root
    pub path: String,
    // pub import_dir: bool,
    pub content: Option<String>,
}


impl From<CreateFileInfo> for MegaNode {
    fn from(value: CreateFileInfo) -> Self {
        MegaNode {
            name: value.name,
            path: value.path.parse().unwrap(),
            is_directory: value.is_directory,
            children: RefCell::new(vec![]),
            id: SHA1::default(),
            mode: TreeItemMode::Tree,
            commit_id: SHA1::default(),
        }
    }
}
