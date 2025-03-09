use std::str::FromStr;
use std::{collections::HashSet, path::PathBuf};

use crate::internal::branch::Branch;
use crate::internal::head::Head;
use crate::utils::client_storage::ClientStorage;
use crate::utils::path;
use crate::utils::util;
use clap::Parser;
use common::utils::{check_conventional_commits_message, format_commit_msg};
use mercury::hash::SHA1;
use mercury::internal::index::Index;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use mercury::internal::object::ObjectTrait;

use super::save_object;

#[derive(Parser, Debug)]
pub struct CommitArgs {
    #[arg(short, long)]
    pub message: String,

    /// allow commit with empty index
    #[arg(long)]
    pub allow_empty: bool,

    /// check if commit message follows conventional commits
    #[arg(long, requires("message"))]
    pub conventional: bool,
}

pub async fn execute(args: CommitArgs) {
    /* check args */
    let index = Index::load(path::index()).unwrap();
    let storage = ClientStorage::init(path::objects());
    let tracked_entries = index.tracked_entries(0);
    if tracked_entries.is_empty() && !args.allow_empty {
        println!("fatal: no changes added to commit, use --allow-empty to override");
        return;
    }
    if args.conventional && !check_conventional_commits_message(&args.message) {
        println!("fatal: commit message does not follow conventional commits");
        return;
    }

    /* Create tree */
    let tree = create_tree(&index, &storage, "".into()).await;

    /* Create & save commit objects */
    let parents_commit_ids = get_parents_ids().await;
    // There must be a `blank line`(\n) before `message`, or remote unpack failed
    let commit = Commit::from_tree_id(
        tree.id,
        parents_commit_ids,
        &format_commit_msg(&args.message, None),
    );

    // TODO  default signature created in `from_tree_id`, wait `git config` to set correct user info

    storage
        .put(&commit.id, &commit.to_data().unwrap(), commit.get_type())
        .unwrap();

    /* update HEAD */
    update_head(&commit.id.to_string()).await;
}

/// recursively create tree from index's tracked entries
async fn create_tree(index: &Index, storage: &ClientStorage, current_root: PathBuf) -> Tree {
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
            Tree::from_bytes(&[], SHA1::default()).unwrap()
        } else {
            Tree::from_tree_items(tree_items).unwrap()
        }
    };
    // save
    save_object(&tree, &tree.id).unwrap();
    tree
}

/// get current head commit id as parent, if in branch, get branch's commit id, if detached head, get head's commit id
async fn get_parents_ids() -> Vec<SHA1> {
    // let current_commit_id = reference::Model::current_commit_hash(db).await.unwrap();
    let current_commit_id = Head::current_commit().await;
    match current_commit_id {
        Some(id) => vec![id],
        None => vec![], // first commit
    }
}

/// update HEAD to new commit, if in branch, update branch's commit id, if detached head, update head's commit id
async fn update_head(commit_id: &str) {
    // let head = reference::Model::current_head(db).await.unwrap();
    match Head::current().await {
        Head::Branch(name) => {
            // in branch
            Branch::update_branch(&name, commit_id, None).await;
        }
        // None => {
        Head::Detached(_) => {
            let head = Head::Detached(SHA1::from_str(commit_id).unwrap());
            Head::update(head, None).await;
        }
    }
}

#[cfg(test)]
mod test {
    use mercury::internal::object::ObjectTrait;

    use crate::{
        command::{add::AddArgs, load_object},
        utils::test,
    };

    use super::*;
    #[test]
    fn test_parse_args() {
        let args = CommitArgs::try_parse_from(["commit", "-m", "init"]);
        assert!(args.is_ok());

        let args = CommitArgs::try_parse_from(["commit", "-m", "init", "--allow-empty"]);
        assert!(args.is_ok());

        let args = CommitArgs::try_parse_from(["commit", "--conventional", "-m", "init"]);
        assert!(args.is_ok());

        let args = CommitArgs::try_parse_from(["commit", "--conventional"]);
        assert!(args.is_err(), "conventional should require message");

        let args = CommitArgs::try_parse_from(["commit"]);
        assert!(args.is_err(), "message is required");
    }

    #[tokio::test]
    async fn test_create_tree() {
        test::reset_working_dir();
        let index = Index::from_file("../tests/data/index/index-760").unwrap();
        println!("{:?}", index.tracked_entries(0).len());
        test::setup_with_new_libra().await;
        let storage = ClientStorage::init(path::objects());
        let tree = create_tree(&index, &storage, "".into()).await;

        assert!(storage.get(&tree.id).is_ok());
        for item in tree.tree_items.iter() {
            if item.mode == TreeItemMode::Tree {
                assert!(storage.get(&item.id).is_ok());
                // println!("tree: {}", item.name);
                if item.name == "DeveloperExperience" {
                    let sub_tree = storage.get(&item.id).unwrap();
                    let tree = Tree::from_bytes(&sub_tree, item.id).unwrap();
                    assert_eq!(tree.tree_items.len(), 4); // 4 subtree according to the test data
                }
            }
        }
    }

    #[tokio::test]
    #[should_panic]
    #[ignore]
    async fn test_execute_commit_with_empty_index_fail() {
        test::setup_with_new_libra().await;
        let args = CommitArgs {
            message: "init".to_string(),
            allow_empty: false,
            conventional: false,
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
                conventional: false,
            };
            execute(args).await;

            // check head branch exists
            let head = Head::current().await;
            let branch_name = match head {
                Head::Branch(name) => name,
                _ => panic!("head not in branch"),
            };
            let branch = Branch::find_branch(&branch_name, None).await.unwrap();
            let commit: Commit = load_object(&branch.commit).unwrap();

            assert_eq!(commit.message.trim(), "init");
            let branch = Branch::find_branch(&branch_name, None).await.unwrap();
            assert_eq!(branch.commit, commit.id);
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
                conventional: false,
            };
            execute(args).await;

            let commit_id = Head::current_commit().await.unwrap();
            let commit: Commit = load_object(&commit_id).unwrap();
            assert_eq!(
                commit.message.trim(),
                "add some files",
                "{}",
                commit.message
            );

            let pre_commit_id = commit.parent_commit_ids[0];
            let pre_commit: Commit = load_object(&pre_commit_id).unwrap();
            assert_eq!(pre_commit.message.trim(), "init");

            let tree_id = commit.tree_id;
            let tree: Tree = load_object(&tree_id).unwrap();
            assert_eq!(tree.tree_items.len(), 2); // 2 subtree according to the test data
        }
    }
}
