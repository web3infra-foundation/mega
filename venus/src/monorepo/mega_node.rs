use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::tree::{Tree, TreeItemMode};

#[derive(Debug, Clone, PartialEq)]
pub struct MegaNode {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub children: RefCell<Vec<Rc<MegaNode>>>,
    pub id: SHA1,
    pub mode: TreeItemMode,
    pub commit_id: SHA1,
}

impl MegaNode {
    pub fn add_child(&self, node: &Rc<MegaNode>) {
        self.children.borrow_mut().push(Rc::clone(node))
    }

    // pub fn convert_to_mega_tree(&self, tree: &Tree) {
    //     let mut model: mega_tree::Model = self.to_owned().into();
    //     model.tree_id = tree.id.to_plain_str();
    //     model.sub_trees = tree.to_data().unwrap();
    //     model.size = 0;
    // }
}

// Entity <==> Node <==> Git Objects
// impl From<MegaNode> for mega_snapshot::Model {
//     fn from(value: MegaNode) -> Self {
//         mega_snapshot::Model {
//             id: generate_id(),
//             path: value.path.to_str().unwrap().to_owned(),
//             name: value.name,
//             import_dir: value.import_dir,
//             tree_id: None,
//             sub_trees: None,
//             commit_id: None,
//             size: 0,
//             created_at: chrono::Utc::now().naive_utc(),
//             updated_at: chrono::Utc::now().naive_utc(),
//         }
//     }
// }

// impl From<MegaNode> for mega_tree::Model {
//     fn from(value: MegaNode) -> Self {
//         mega_tree::Model {
//             id: generate_id(),
//             repo_id: 0,
//             full_path: value.path.to_str().unwrap().to_owned(),
//             tree_id: String::new(),
//             sub_trees: Vec::new(),
//             name: String::new(),
//             parent_id: None,
//             size: 0,
//             commit_id: String::new(),
//             created_at: chrono::Utc::now().naive_utc(),
//             updated_at: chrono::Utc::now().naive_utc(),
//         }
//     }
// }

impl From<Tree> for MegaNode {
    fn from(value: Tree) -> Self {
        MegaNode {
            name: String::new(),
            path: PathBuf::new(),
            is_directory: true,
            children: RefCell::new(vec![]),
            id: value.id,
            mode: TreeItemMode::Tree,
            commit_id: SHA1::default(),
        }
    }
}

impl From<Blob> for MegaNode {
    fn from(value: Blob) -> Self {
        MegaNode {
            name: String::new(),
            path: PathBuf::new(),
            is_directory: false,
            children: RefCell::new(vec![]),
            id: value.id,
            mode: TreeItemMode::Blob,
            commit_id: SHA1::default(),
        }
    }
}
