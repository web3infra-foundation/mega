use std::{collections::HashSet, path::PathBuf};

use clap::Parser;
use storage::driver::file_storage::{local_storage::LocalStorage, FileStorage};
use venus::internal::object::tree::{Tree, TreeItem, TreeItemMode};

use crate::{internal::index::Index, utils::util};

#[derive(Parser, Debug)]
#[command(about = "Record changes to the repository")]
pub struct CommitArgs {
    #[arg(short, long)]
    pub message: String,

    #[arg(long)]
    pub allow_empty: bool,
}

async fn create_tree(index: &Index, storage: &dyn FileStorage, current_root: PathBuf) -> Tree {
    // blob created when add file to index
    let get_blob_entry = |path: &PathBuf| {
        let name = util::path_to_string(path);
        let mete = index.get(&name, 0).unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap().to_string();

        TreeItem {
            name: filename,
            mode: TreeItemMode::tree_item_type_from_bytes(format!("{:o}", mete.mode).as_bytes())
                .unwrap(),
            id: mete.hash,
        }
    };

    let mut tree_items: Vec<TreeItem> = Vec::new();
    let mut processed_path: HashSet<String> = HashSet::new();
    let path_entries: Vec<PathBuf> = index
        .tracked_entries(0)
        .iter()
        .map(|file| PathBuf::from(file.name.clone()))
        .filter(|path| path.starts_with(&current_root))
        .collect();
    for path in path_entries.iter() {
        // check if the file is in the current root
        let in_path = path.parent().unwrap() == current_root;
        if in_path {
            let item = get_blob_entry(path);
            tree_items.push(item);
        } else {
            if path.components().count() == 1 {
                continue;
            }
            // 拿到下一级别目录
            let process_path = path
                .components()
                .nth(current_root.components().count())
                .unwrap()
                .as_os_str()
                .to_str()
                .unwrap();

            if processed_path.contains(process_path) {
                continue;
            }
            processed_path.insert(process_path.to_string());

            let sub_tree = Box::pin(create_tree(
                index,
                storage,
                current_root.clone().join(process_path),
            ))
            .await;
            tree_items.push(TreeItem {
                name: process_path.to_string(),
                mode: TreeItemMode::Tree,
                id: sub_tree.id,
            });
        }
    }
    let tree = Tree::from_tree_items(tree_items).unwrap();
    // save
    let data = tree.to_data().unwrap();
    storage
        .put(&tree.id.to_plain_str(), data.len() as i64, &data)
        .await
        .unwrap();
    tree
}

pub async fn execute(args: CommitArgs) {
    let index = Index::from_file(util::working_dir().join("index")).unwrap();
    let storage = LocalStorage::init(util::storage_path().join("objects"));
    let tracked_entries = index.tracked_entries(0);
    if tracked_entries.is_empty() && !args.allow_empty {
        panic!("fatal: no changes added to commit, use --allow-empty to override");
    }

    let tree = create_tree(&index, &storage, "".into()).await;
    // TODO wait for head & status
}

#[cfg(test)]
mod test {
    use venus::internal::object::ObjectTrait;

    use crate::utils::test;

    use super::*;

    #[tokio::test]
    async fn test_create_tree() {
        let index = Index::from_file("../tests/data/index/index-760").unwrap();
        println!("{:?}", index.tracked_entries(0).len());
        test::setup_with_new_libra().await;
        let storage = LocalStorage::init(util::storage_path().join("objects"));
        let tree = create_tree(&index, &storage, "".into()).await;

        assert!(storage.get(&tree.id.to_plain_str()).await.is_ok());
        for item in tree.tree_items.iter() {
            if item.mode == TreeItemMode::Tree {
                assert!(storage.get(&item.id.to_plain_str()).await.is_ok());
                // println!("tree: {}", item.name);
                if item.name == "DeveloperExperience" {
                    let sub_tree = storage.get(&item.id.to_plain_str()).await.unwrap();
                    let tree = Tree::from_bytes(sub_tree.to_vec(), item.id).unwrap();
                    assert!(tree.tree_items.len() == 4); // 4 sub tree according to the test data
                }
            }
        }
    }
}
