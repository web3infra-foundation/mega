use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;

use callisto::{mega_blob, mega_tree};
use venus::hash::SHA1;
use venus::internal::object::blob::Blob;
use venus::internal::object::commit::Commit;
use venus::internal::object::signature::Signature;
use venus::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use venus::internal::object::types::ObjectType;
use venus::internal::object::{utils, ObjectTrait};
use venus::internal::zlib::stream::inflate::ReadBoxed;

pub fn generate_git_keep() -> Blob {
    let git_keep_content = String::from("This file was used to maintain the git tree");
    let blob_content = Cursor::new(utils::compress_zlib(git_keep_content.as_bytes()).unwrap());
    let mut buf = ReadBoxed::new(blob_content, ObjectType::Blob, git_keep_content.len());
    Blob::from_buf_read(&mut buf, git_keep_content.len())
}

pub fn init_commit(tree_id: SHA1) -> Commit {
    let author = Signature::from_data(
        "author benjamin.747 <benjamin.747@outlook.com> 1709263583 +0800"
            .to_string()
            .into_bytes(),
    )
    .unwrap();
    let committer = author.clone();
    let mut commit = Commit {
        id: SHA1::default(),
        tree_id,
        parent_commit_ids: vec![],
        author,
        committer,
        message: String::from("Init Mega Directory"),
    };
    let hash = SHA1::from_type_and_data(ObjectType::Commit, &commit.to_data().unwrap());
    commit.id = hash;
    commit
}

pub fn init_trees(git_keep: &Blob) -> (HashMap<SHA1, Tree>, Tree) {
    let rust_item = TreeItem {
        mode: TreeItemMode::Blob,
        id: git_keep.id,
        name: String::from(".gitkeep"),
    };
    let rust = Tree::from_tree_items(vec![rust_item]).unwrap();
    let imports = rust.clone();
    let projects_items = vec![
        TreeItem {
            mode: TreeItemMode::Blob,
            id: git_keep.id,
            name: String::from(".gitkeep"),
        },
        TreeItem {
            mode: TreeItemMode::Tree,
            id: rust.id,
            name: String::from("rust"),
        },
    ];
    let projects = Tree::from_tree_items(projects_items).unwrap();
    let root_items = vec![
        TreeItem {
            mode: TreeItemMode::Blob,
            id: git_keep.id,
            name: String::from(".gitkeep"),
        },
        TreeItem {
            mode: TreeItemMode::Tree,
            id: imports.id,
            name: String::from("imports"),
        },
        TreeItem {
            mode: TreeItemMode::Tree,
            id: projects.id,
            name: String::from("projects"),
        },
    ];

    let root = Tree::from_tree_items(root_items).unwrap();
    let trees = vec![imports, projects, rust];
    (trees.into_iter().map(|x| (x.id, x)).collect(), root)
}

pub struct MegaModelConverter {
    pub commit: Commit,
    pub root_tree: Tree,
    pub tree_maps: HashMap<SHA1, Tree>,
    pub blob_maps: HashMap<SHA1, Blob>,
    pub mega_trees: RefCell<Vec<mega_tree::ActiveModel>>,
    pub mega_blobs: RefCell<Vec<mega_blob::ActiveModel>>,
    pub current_path: RefCell<PathBuf>,
}

impl MegaModelConverter {
    pub fn traverse_tree(&self, tree: &Tree) {
        for item in &tree.tree_items {
            let name = item.name.clone();
            self.current_path.borrow_mut().push(&name);
            if item.mode == TreeItemMode::Tree {
                let child_tree = self.tree_maps.get(&item.id).unwrap();
                let mut mega_tree: mega_tree::Model = child_tree.to_owned().into();
                mega_tree.full_path = self.current_path.borrow().to_str().unwrap().to_owned();
                mega_tree.name = name;
                mega_tree.commit_id = self.commit.id.to_plain_str();
                self.mega_trees.borrow_mut().push(mega_tree.into());
                self.traverse_tree(child_tree);
            } else {
                let blob = self.blob_maps.get(&item.id).unwrap();
                let mut mega_blob: mega_blob::Model = blob.to_owned().into();
                mega_blob.full_path = self.current_path.borrow().to_str().unwrap().to_owned();
                mega_blob.name = name;
                mega_blob.commit_id = self.commit.id.to_plain_str();
                self.mega_blobs.borrow_mut().push(mega_blob.into());
            }
            self.current_path.borrow_mut().pop();
        }
    }

    pub fn init() -> Self {
        let git_keep = generate_git_keep();
        let (tree_maps, root_tree) = init_trees(&git_keep);
        let commit = init_commit(root_tree.id);
        let mut blob_maps = HashMap::new();
        blob_maps.insert(git_keep.id, git_keep);

        MegaModelConverter {
            commit,
            root_tree,
            tree_maps,
            blob_maps,
            mega_trees: RefCell::new(vec![]),
            mega_blobs: RefCell::new(vec![]),
            current_path: RefCell::new(PathBuf::from("/")),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::util::MegaModelConverter;

    #[test]
    pub fn test_init_mega_dir() {
        let converter = MegaModelConverter::init();
        converter.traverse_tree(&converter.root_tree);
        let mega_trees = converter.mega_trees.borrow().clone();
        let mega_blobs = converter.mega_blobs.borrow().clone();

        for i in mega_trees {
            println!("{:?}", i);
        }
        for i in mega_blobs {
            println!("{:?}", i);
        }
    }
}
