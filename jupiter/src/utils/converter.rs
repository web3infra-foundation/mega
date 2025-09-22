use std::cell::RefCell;
use std::collections::HashMap;

use callisto::{mega_blob, mega_refs, mega_tree, raw_blob};
use common::config::MonoConfig;
use common::utils::{generate_id, MEGA_BRANCH_NAME};
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};

pub fn generate_git_keep() -> Blob {
    let git_keep_content = String::from("This file was used to maintain the git tree");
    Blob::from_content(&git_keep_content)
}

pub fn generate_git_keep_with_timestamp() -> Blob {
    let git_keep_content = format!(
        "This file was used to maintain the git tree, generate at:{}",
        chrono::Utc::now().naive_utc()
    );
    Blob::from_content(&git_keep_content)
}

pub fn init_trees(mono_config: &MonoConfig) -> (HashMap<SHA1, Tree>, HashMap<SHA1, Blob>, Tree) {
    let mut root_items = Vec::new();
    let mut trees = Vec::new();
    let mut blobs = Vec::new();
    for dir in mono_config.root_dirs.clone() {
        let entity_str =
            saturn::entitystore::generate_entity(&mono_config.admin, &format!("/{dir}")).unwrap();
        let blob = Blob::from_content(&entity_str);

        let tree_item = TreeItem {
            mode: TreeItemMode::Blob,
            id: blob.id,
            name: String::from(".mega_cedar.json"),
        };
        let tree = Tree::from_tree_items(vec![tree_item.clone()]).unwrap();
        root_items.push(TreeItem {
            mode: TreeItemMode::Tree,
            id: tree.id,
            name: dir,
        });
        trees.push(tree);
        blobs.push(blob);
    }

    let root = Tree::from_tree_items(root_items).unwrap();
    (
        trees.into_iter().map(|x| (x.id, x)).collect(),
        blobs.into_iter().map(|x| (x.id, x)).collect(),
        root,
    )
}

pub struct MegaModelConverter {
    pub commit: Commit,
    pub root_tree: Tree,
    pub tree_maps: HashMap<SHA1, Tree>,
    pub blob_maps: HashMap<SHA1, Blob>,
    pub mega_trees: RefCell<HashMap<SHA1, mega_tree::ActiveModel>>,
    pub mega_blobs: RefCell<HashMap<SHA1, mega_blob::ActiveModel>>,
    pub raw_blobs: RefCell<HashMap<SHA1, raw_blob::ActiveModel>>,
    pub refs: mega_refs::ActiveModel,
}

impl MegaModelConverter {
    fn traverse_from_root(&self) {
        let root_tree = &self.root_tree;
        let sea_tree: mercury::internal::model::sea_models::mega_tree::Model =
            mercury::internal::model::sea_models::mega_tree::Model::from(root_tree.to_owned());
        let mut mega_tree: mega_tree::Model = callisto::mega_tree::Model::from(sea_tree);
        mega_tree.commit_id = self.commit.id.to_string();
        self.mega_trees
            .borrow_mut()
            .insert(root_tree.id, mega_tree.clone().into());
        self.traverse_for_update(&self.root_tree);
    }

    fn traverse_for_update(&self, tree: &Tree) {
        for item in &tree.tree_items {
            if item.mode == TreeItemMode::Tree {
                let child_tree = self.tree_maps.get(&item.id).unwrap();
                let sea_tree: mercury::internal::model::sea_models::mega_tree::Model =
                    mercury::internal::model::sea_models::mega_tree::Model::from(
                        child_tree.to_owned(),
                    );
                let mut mega_tree: mega_tree::Model = callisto::mega_tree::Model::from(sea_tree);
                mega_tree.commit_id = self.commit.id.to_string();
                self.mega_trees
                    .borrow_mut()
                    .insert(child_tree.id, mega_tree.clone().into());
                self.traverse_for_update(child_tree);
            } else {
                let blob = self.blob_maps.get(&item.id).unwrap();
                let sea_blob: mercury::internal::model::sea_models::mega_blob::Model =
                    mercury::internal::model::sea_models::mega_blob::Model::from(blob);
                let mut mega_blob: mega_blob::Model = callisto::mega_blob::Model::from(sea_blob);
                mega_blob.commit_id = self.commit.id.to_string();
                self.mega_blobs
                    .borrow_mut()
                    .insert(blob.id, mega_blob.clone().into());
                let sea_raw: mercury::internal::model::sea_models::raw_blob::Model =
                    mercury::internal::model::sea_models::raw_blob::Model::from(blob);
                let raw_blob: raw_blob::Model = callisto::raw_blob::Model::from(sea_raw);
                self.raw_blobs.borrow_mut().insert(blob.id, raw_blob.into());
            }
        }
    }

    pub fn init(mono_config: &MonoConfig) -> Self {
        let (tree_maps, blob_maps, root_tree) = init_trees(mono_config);
        let commit = Commit::from_tree_id(root_tree.id, vec![], "\nInit Mega Directory");

        let mega_ref = mega_refs::Model {
            id: generate_id(),
            path: "/".to_owned(),
            ref_name: MEGA_BRANCH_NAME.to_owned(),
            ref_commit_hash: commit.id.to_string(),
            ref_tree_hash: commit.tree_id.to_string(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            is_mr: false,
        };

        let converter = MegaModelConverter {
            commit,
            root_tree,
            tree_maps,
            blob_maps,
            mega_trees: RefCell::new(HashMap::new()),
            mega_blobs: RefCell::new(HashMap::new()),
            raw_blobs: RefCell::new(HashMap::new()),
            refs: mega_ref.into(),
        };
        converter.traverse_from_root();
        converter
    }
}

#[cfg(test)]
mod test {

    use std::str::FromStr;

    use common::config::MonoConfig;
    use mercury::{hash::SHA1, internal::object::commit::Commit};

    use crate::utils::converter::MegaModelConverter;

    #[test]
    pub fn test_init_mega_dir() {
        let mono_config = MonoConfig::default();
        let converter = MegaModelConverter::init(&mono_config);
        let mega_trees = converter.mega_trees.borrow().clone();
        let mega_blobs = converter.mega_blobs.borrow().clone();
        let raw_blob = converter.raw_blobs.borrow().clone();
        let dir_nums = mono_config.root_dirs.len();
        assert_eq!(mega_trees.len(), dir_nums + 1);
        assert_eq!(mega_blobs.len(), dir_nums);
        assert_eq!(raw_blob.len(), dir_nums);
    }

    #[test]
    pub fn test_init_commit() {
        let commit = Commit::from_tree_id(
            SHA1::from_str("bd4a28f2d8b2efc371f557c3b80d320466ed83f3").unwrap(),
            vec![],
            "\nInit Mega Directory",
        );
        println!("{commit}");
    }
}
