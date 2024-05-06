use std::path::{Path, PathBuf};

use venus::hash::SHA1;
use venus::internal::object::blob::Blob;
use venus::internal::object::commit::Commit;
use venus::internal::object::ObjectTrait;
use venus::internal::object::tree::{Tree, TreeItemMode};

use crate::utils::util;

pub trait TreeExt {
    fn load(hash: &SHA1) -> Tree;
    fn get_plain_items(&self) -> Vec<(PathBuf, SHA1)>;
}

pub trait CommitExt {
    fn load(hash: &SHA1) -> Commit;
}

pub trait BlobExt {
    fn load(hash: &SHA1) -> Blob;
    fn from_file(path: impl AsRef<Path>) -> Blob;
    fn save(&self) -> SHA1;
}

impl TreeExt for Tree {
    fn load(hash: &SHA1) -> Tree {
        let storage = util::objects_storage();
        let tree_data = storage.get(hash).unwrap();
        Tree::from_bytes(tree_data.to_vec(), *hash).unwrap()
    }

    /// Get all the items in the tree recursively (to workdir path)
    fn get_plain_items(&self) -> Vec<(PathBuf, SHA1)> {
        let mut items = Vec::new();
        for item in self.tree_items.iter() {
            if item.mode == TreeItemMode::Blob {
                items.push((PathBuf::from(item.name.clone()), item.id));
            } else {
                let sub_tree = Tree::load(&item.id);
                let sub_entries = sub_tree.get_plain_items();

                items.append(
                    sub_entries
                        .iter()
                        .map(|(path, hash)| (PathBuf::from(item.name.clone()).join(path), *hash))
                        .collect::<Vec<(PathBuf, SHA1)>>()
                        .as_mut(),
                );
            }
        }
        items
    }
}

impl CommitExt for Commit {
    fn load(hash: &SHA1) -> Commit {
        let storage = util::objects_storage();
        let commit_data = storage.get(hash).unwrap();
        Commit::from_bytes(commit_data.to_vec(), *hash).unwrap()
    }
}

impl BlobExt for Blob {
    fn load(hash: &SHA1) -> Blob {
        let storage = util::objects_storage();
        let blob_data = storage.get(hash).unwrap();
        Blob::from_bytes(blob_data, *hash).unwrap()
    }

    /// Create a blob from a file
    /// - `path`: absolute  or relative path to current dir
    fn from_file(path: impl AsRef<Path>) -> Blob {
        let file_content = std::fs::read_to_string(path).unwrap();
        Blob::from_content(&file_content)
    }

    fn save(&self) -> SHA1 {
        let storage = util::objects_storage();
        let id = self.id;
        if !storage.exist(&id) {
            storage.put(&id, &self.data, self.get_type()).unwrap();
        }
        self.id
    }
}