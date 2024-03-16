use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use callisto::db_enums::RefType;
use callisto::{mega_blob, mega_snapshot, mega_tree, raw_blob, refs};
use common::utils::{generate_id, MEGA_BRANCH_NAME};
use venus::hash::SHA1;
use venus::internal::object::blob::Blob;
use venus::internal::object::commit::Commit;
use venus::internal::object::signature::Signature;
use venus::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use venus::internal::object::types::ObjectType;

pub fn generate_git_keep() -> Blob {
    let git_keep_content = String::from("This file was used to maintain the git tree");
    Blob::from_content(&git_keep_content)
}

pub fn init_commit(tree_id: SHA1, parent_commit_ids: Vec<SHA1>, message: &str) -> Commit {
    let author = Signature::from_data(
        format!(
            "author benjamin.747 <benjamin.747@outlook.com> {} +0800",
            chrono::Utc::now().timestamp()
        )
        .to_string()
        .into_bytes(),
    )
    .unwrap();
    let committer = author.clone();
    let mut commit = Commit {
        id: SHA1::default(),
        tree_id,
        parent_commit_ids,
        author,
        committer,
        message: message.to_string(),
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
    let import = rust.clone();
    let project_items = vec![TreeItem {
        mode: TreeItemMode::Tree,
        id: rust.id,
        name: String::from("rust"),
    }];
    let project = Tree::from_tree_items(project_items).unwrap();
    let root_items = vec![
        TreeItem {
            mode: TreeItemMode::Tree,
            id: import.id,
            name: String::from("third-part"),
        },
        TreeItem {
            mode: TreeItemMode::Tree,
            id: project.id,
            name: String::from("project"),
        },
    ];

    let root = Tree::from_tree_items(root_items).unwrap();
    let trees = vec![import, project, rust];
    (trees.into_iter().map(|x| (x.id, x)).collect(), root)
}

pub struct MegaModelConverter {
    pub commit: Commit,
    pub root_tree: Tree,
    pub tree_maps: HashMap<SHA1, Tree>,
    pub blob_maps: HashMap<SHA1, Blob>,
    pub mega_trees: RefCell<Vec<mega_tree::ActiveModel>>,
    pub mega_blobs: RefCell<Vec<mega_blob::ActiveModel>>,
    pub mega_snapshots: RefCell<Vec<mega_snapshot::ActiveModel>>,
    pub raw_blobs: RefCell<HashMap<SHA1, raw_blob::ActiveModel>>,
    pub refs: refs::ActiveModel,
    pub current_path: RefCell<PathBuf>,
}

impl MegaModelConverter {
    fn traverse_from_root(&self) {
        let root_tree = &self.root_tree;
        let mut mega_tree: mega_tree::Model = root_tree.to_owned().into();
        mega_tree.full_path = self.current_path.borrow().to_str().unwrap().to_owned();
        mega_tree.name = String::from("root");
        mega_tree.commit_id = self.commit.id.to_plain_str();
        self.mega_trees.borrow_mut().push(mega_tree.clone().into());
        let snapshot: mega_snapshot::Model = mega_tree.into();
        self.mega_snapshots.borrow_mut().push(snapshot.into());
        self.traverse_for_update(&self.root_tree);
    }

    fn traverse_for_update(&self, tree: &Tree) {
        for item in &tree.tree_items {
            let name = item.name.clone();
            self.current_path.borrow_mut().push(&name);
            if item.mode == TreeItemMode::Tree {
                let child_tree = self.tree_maps.get(&item.id).unwrap();
                let mut mega_tree: mega_tree::Model = child_tree.to_owned().into();
                mega_tree.full_path = self.current_path.borrow().to_str().unwrap().to_owned();
                mega_tree.name = name;
                mega_tree.commit_id = self.commit.id.to_plain_str();
                mega_tree.parent_id = Some(tree.id.to_plain_str());
                self.mega_trees.borrow_mut().push(mega_tree.clone().into());
                let snapshot: mega_snapshot::Model = mega_tree.into();
                self.mega_snapshots.borrow_mut().push(snapshot.into());
                self.traverse_for_update(child_tree);
            } else {
                let blob = self.blob_maps.get(&item.id).unwrap();
                let mut mega_blob: mega_blob::Model = blob.to_owned().into();
                mega_blob.full_path = self.current_path.borrow().to_str().unwrap().to_owned();
                mega_blob.name = name;
                mega_blob.commit_id = self.commit.id.to_plain_str();
                self.mega_blobs.borrow_mut().push(mega_blob.clone().into());
                let raw_blob: raw_blob::Model = blob.to_owned().into();
                self.raw_blobs.borrow_mut().insert(blob.id, raw_blob.into());
                let snapshot: mega_snapshot::Model = mega_blob.into();
                self.mega_snapshots.borrow_mut().push(snapshot.into());
            }
            self.current_path.borrow_mut().pop();
        }
    }

    pub fn init() -> Self {
        let git_keep = generate_git_keep();
        let (tree_maps, root_tree) = init_trees(&git_keep);
        let commit = init_commit(root_tree.id, vec![], "Init Mega Directory");
        let mut blob_maps = HashMap::new();
        blob_maps.insert(git_keep.id, git_keep);

        let mega_ref = refs::Model {
            id: generate_id(),
            repo_id: 0,
            ref_name: String::from(MEGA_BRANCH_NAME),
            ref_git_id: commit.id.to_plain_str(),
            ref_type: RefType::Branch,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };

        let converter  = MegaModelConverter {
            commit,
            root_tree,
            tree_maps,
            blob_maps,
            mega_trees: RefCell::new(vec![]),
            mega_blobs: RefCell::new(vec![]),
            mega_snapshots: RefCell::new(vec![]),
            raw_blobs: RefCell::new(HashMap::new()),
            refs: mega_ref.into(),
            current_path: RefCell::new(PathBuf::from("/")),
        };
        converter.traverse_from_root();
        converter
    }
}

#[cfg(test)]
mod test {

    use std::str::FromStr;

    use venus::hash::SHA1;

    use crate::model::converter::{init_commit, MegaModelConverter};

    #[test]
    pub fn test_init_mega_dir() {
        let converter = MegaModelConverter::init();
        let mega_trees = converter.mega_trees.borrow().clone();
        let mega_blobs = converter.mega_blobs.borrow().clone();
        let raw_blob = converter.raw_blobs.borrow().clone();
        let snapshot = converter.mega_snapshots.borrow().clone();
        assert_eq!(mega_trees.len(), 4);
        assert_eq!(mega_blobs.len(), 2);
        assert_eq!(raw_blob.len(), 1);
        assert_eq!(snapshot.len(), 6);
        for i in snapshot {
            println!("{:?}", i.full_path);
        }
    }

    #[test]
    pub fn test_init_commit() {
        let commit = init_commit(
            SHA1::from_str("bd4a28f2d8b2efc371f557c3b80d320466ed83f3").unwrap(),
            vec![],
            "Init Mega Directory",
        );
        println!("{}", commit);
    }
}
