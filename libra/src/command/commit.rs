use std::str::FromStr;
use std::{collections::HashSet, path::PathBuf};

use crate::db::get_db_conn;
use crate::model::reference;
use crate::model::reference::ActiveModel;
use crate::utils::path;
use crate::{internal::index::Index, utils::util};
use clap::Parser;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{ActiveModelTrait, Set};
use storage::driver::file_storage::{local_storage::LocalStorage, FileStorage};
use venus::hash::SHA1;
use venus::internal::object::commit::Commit;
use venus::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use venus::internal::object::ObjectTrait;

#[derive(Parser, Debug)]
#[command(about = "Record changes to the repository")]
pub struct CommitArgs {
    #[arg(short, long)]
    pub message: String,

    #[arg(long)]
    pub allow_empty: bool,
}

pub async fn execute(args: CommitArgs) {
    /* check args */
    let index = Index::load().unwrap();
    let storage = LocalStorage::init(path::objects());
    let tracked_entries = index.tracked_entries(0);
    if tracked_entries.is_empty() && !args.allow_empty {
        panic!("fatal: no changes added to commit, use --allow-empty to override");
    }

    /* Create tree */
    let tree = create_tree(&index, &storage, "".into()).await;
    let db = get_db_conn().await.unwrap();

    /* Create & save commit objects */
    let parents_commit_ids = get_parents_ids(&db).await;
    let commit = Commit::from_tree_id(tree.id, parents_commit_ids, args.message.as_str());

    // TODO  default signature created in `from_tree_id`, wait `git config` to set correct user info

    storage
        .put(
            &commit.id.to_plain_str(),
            commit.to_data().unwrap().len() as i64,
            &commit.to_data().unwrap(),
        )
        .await
        .unwrap();

    /* update HEAD */
    update_head(&db, &commit.id.to_plain_str()).await;
}

/// recursively create tree from index's tracked entries
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
        let in_current_path = path.parent().unwrap() == current_root;
        if in_current_path {
            let item = get_blob_entry(path);
            tree_items.push(item);
        } else {
            if path.components().count() == 1 {
                continue;
            }
            // next level tree
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
    let tree = {
        // `from_tree_items` can't create empty tree, so use `from_bytes` instead
        if tree_items.is_empty() {
            // git create a no zero hash for empty tree, didn't know method. use default SHA1 temporarily
            Tree::from_bytes(vec![], SHA1::default()).unwrap()
        } else {
            Tree::from_tree_items(tree_items).unwrap()
        }
    };
    // save
    let data = tree.to_data().unwrap();
    storage
        .put(&tree.id.to_plain_str(), data.len() as i64, &data)
        .await
        .unwrap();
    tree
}

/// get current head commit id as parent, if in branch, get branch's commit id, if detached head, get head's commit id
async fn get_parents_ids(db: &sea_orm::DbConn) -> Vec<SHA1> {
    let head = reference::Model::current_head(db).await.unwrap();
    match head.name {
        Some(name) => {
            let commit = reference::Model::find_branch_by_name(db, name.as_str())
                .await
                .unwrap();
            match commit {
                Some(commit) => vec![SHA1::from_str(commit.commit.unwrap().as_str()).unwrap()],
                None => vec![], // empty branch, first commit
            }
        }
        None => vec![SHA1::from_str(head.commit.unwrap().as_str()).unwrap()],
    }
}

/// update HEAD to new commit, if in branch, update branch's commit id, if detached head, update head's commit id
async fn update_head(db: &sea_orm::DbConn, commit_id: &str) {
    let head = reference::Model::current_head(db).await.unwrap();

    match head.name {
        Some(name) => {
            // in branch
            let branch = reference::Model::find_branch_by_name(db, name.as_str())
                .await
                .unwrap();
            match branch {
                Some(branch) => {
                    let mut branch: ActiveModel = branch.into();
                    branch.commit = Set(Some(commit_id.to_string()));
                    branch.update(db).await.unwrap();
                }
                None => {
                    // branch not found, create new branch
                    let new_branch = reference::ActiveModel {
                        id: NotSet,
                        name: Set(Some(name.clone())),
                        kind: Set(reference::ConfigKind::Branch),
                        commit: Set(Some(commit_id.to_string())),
                        remote: Set(None),
                    };
                    new_branch.save(db).await.unwrap();
                }
            }
        }
        None => {
            // detached head
            let mut head: ActiveModel = head.into();
            head.commit = Set(Some(commit_id.to_string()));
            head.update(db).await.unwrap();
        }
    }
}

#[cfg(test)]
mod test {
    use venus::internal::object::ObjectTrait;

    use crate::{
        command::{add::AddArgs, load_object},
        utils::test,
    };

    use super::*;

    #[tokio::test]
    async fn test_create_tree() {
        let index = Index::from_file("../tests/data/index/index-760").unwrap();
        println!("{:?}", index.tracked_entries(0).len());
        test::setup_with_new_libra().await;
        let storage = LocalStorage::init(path::objects());
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

    #[tokio::test]
    #[should_panic]
    async fn test_excute_commit_with_empty_index_fail() {
        test::setup_with_new_libra().await;
        let args = CommitArgs {
            message: "init".to_string(),
            allow_empty: false,
        };
        execute(args).await;
    }

    #[tokio::test]
    async fn test_execute_commit() {
        test::setup_with_new_libra().await;
        // create first empty commit
        {
            let args = CommitArgs {
                message: "init".to_string(),
                allow_empty: true,
            };
            execute(args).await;

            let db = get_db_conn().await.unwrap();
            // check head branch exists
            let head = reference::Model::current_head(&db).await.unwrap();
            let branch = reference::Model::find_branch_by_name(&db, &head.name.unwrap())
                .await
                .unwrap();
            assert!(branch.is_some());
            let commit_id = branch.unwrap().commit.unwrap();
            let storage = LocalStorage::init(path::objects());
            let commit: Commit = load_object(&commit_id, &storage).await.unwrap();

            assert!(commit.message == "init");
            db.close().await.unwrap();
        }
        // create a new commit
        {
            // create `a.txt` `bb/b.txt` `bb/c.txt`
            test::ensure_file("a.txt", Some("a"));
            test::ensure_file("bb/b.txt", Some("b"));
            test::ensure_file("bb/c.txt", Some("c"));
            let args = AddArgs {
                all: true,
                update: false,
                verbose: false,
                pathspec: vec![],
            };
            crate::command::add::execute(args).await;
        }

        {
            let args = CommitArgs {
                message: "add some files".to_string(),
                allow_empty: false,
            };
            execute(args).await;

            let db = get_db_conn().await.unwrap();
            // check head branch exists
            let head = reference::Model::current_head(&db).await.unwrap();
            let branch = reference::Model::find_branch_by_name(&db, &head.name.unwrap())
                .await
                .unwrap();
            let commit_id = branch.unwrap().commit.unwrap();
            let storage = LocalStorage::init(path::objects());
            let commit: Commit = load_object(&commit_id, &storage).await.unwrap();
            assert!(commit.message == "add some files");

            let pre_commit_id = commit.parent_commit_ids[0].to_plain_str();
            let pre_commit: Commit = load_object(&pre_commit_id, &storage).await.unwrap();
            assert!(pre_commit.message == "init");

            let tree_id = commit.tree_id.to_plain_str();
            let tree: Tree = load_object(&tree_id, &storage).await.unwrap();
            assert!(tree.tree_items.len() == 2); // 2 sub tree according to the test data
        }
    }
}
