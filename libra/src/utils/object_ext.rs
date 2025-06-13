use colored::Colorize;
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::{Tree, TreeItemMode};
use mercury::internal::object::ObjectTrait;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::utils::{lfs, util};

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
    fn from_lfs_file(path: impl AsRef<Path>) -> Blob;
    fn save(&self) -> SHA1;
}

impl TreeExt for Tree {
    fn load(hash: &SHA1) -> Tree {
        let storage = util::objects_storage();
        let tree_data = storage.get(hash).unwrap();
        Tree::from_bytes(&tree_data, *hash).unwrap()
    }

    /// Get all the items in the tree recursively (to workdir path)
    fn get_plain_items(&self) -> Vec<(PathBuf, SHA1)> {
        let mut items = Vec::new();
        for item in self.tree_items.iter() {
            if item.mode != TreeItemMode::Tree {
                // Not Tree, maybe Blob, link, etc.
                if item.mode == TreeItemMode::Commit {
                    // submodule
                    eprintln!("{}", "Warning: Submodule is not supported yet".red());
                }
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
        Commit::from_bytes(&commit_data, *hash).unwrap()
    }
}

impl BlobExt for Blob {
    fn load(hash: &SHA1) -> Blob {
        let storage = util::objects_storage();
        let blob_data = storage.get(hash).unwrap();
        Blob::from_bytes(&blob_data, *hash).unwrap()
    }

    /// Create a blob from a file
    /// - `path`: absolute  or relative path to current dir
    fn from_file(path: impl AsRef<Path>) -> Blob {
        let mut data = Vec::new();
        let file = fs::File::open(path).unwrap();
        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut data).unwrap();
        Blob::from_content_bytes(data)
    }

    /// Create a blob from an LFS file
    /// - include: create a pointer file & copy the file to `.libra/lfs/objects`
    /// - `path`: absolute  or relative path to current dir
    fn from_lfs_file(path: impl AsRef<Path>) -> Blob {
        let (pointer, oid) = lfs::generate_pointer_file(&path);
        tracing::debug!("\n{}", pointer);
        lfs::backup_lfs_file(&path, &oid).unwrap();
        Blob::from_content(&pointer)
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
