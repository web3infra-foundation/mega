use std::path::PathBuf;
use storage::driver::file_storage::FileStorage;
use storage::driver::file_storage::local_storage::LocalStorage;
use venus::hash::SHA1;
use venus::internal::object::ObjectTrait;
use venus::internal::object::tree::{Tree, TreeItemMode};
use crate::utils::path;

pub trait TreeExt {
    async fn load(hash: &SHA1) -> Tree;
    async fn get_plain_items(&self) -> Vec<(PathBuf, SHA1)>;
}
impl TreeExt for Tree {
    async fn load(hash: &SHA1) -> Tree {
        let storage = LocalStorage::init(path::objects());
        let tree_data = storage.get(&hash.to_plain_str()).await.unwrap();
        Tree::from_bytes(tree_data.to_vec(), hash.clone()).unwrap()
    }

    async fn get_plain_items(&self) -> Vec<(PathBuf, SHA1)> {
        let mut items = Vec::new();
        for item in self.tree_items.iter() {
            if item.mode == TreeItemMode::Blob {
                items.push((PathBuf::from(item.name.clone()), item.id.clone()));
            } else {
                let sub_tree = Tree::load(&item.id).await;
                // let sub_entries = sub_tree.get_plain_items().await;
                let sub_entries = Box::pin(sub_tree.get_plain_items()).await;
                //TODO 异步递归可能有问题

                items.append(
                    sub_entries
                        .iter()
                        .map(|(path, hash)| (PathBuf::from(item.name.clone()).join(path), hash.clone()))
                        .collect::<Vec<(PathBuf, SHA1)>>()
                        .as_mut(),
                );
            }
        }
        items
    }
}