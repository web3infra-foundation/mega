use super::save_object;
use std::process::Stdio;
use std::str::FromStr;
use std::{collections::HashSet, path::PathBuf};

const EMPTY_TREE_HASH: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

use crate::command::load_object;
use crate::internal::branch::Branch;
use crate::internal::config::Config as UserConfig;
use crate::internal::head::Head;
use crate::internal::reflog::{with_reflog, ReflogAction, ReflogContext};
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
use sea_orm::ConnectionTrait;
use std::process::Command;

#[derive(Parser, Debug, Default)]
pub struct CommitArgs {
    #[arg(short, long)]
    pub message: String,

    /// allow commit with empty index
    #[arg(long)]
    pub allow_empty: bool,

    /// check if the commit message follows conventional commits
    #[arg(long, requires("message"))]
    pub conventional: bool,

    /// amend the last commit
    #[arg(long)]
    pub amend: bool,

    /// add signed-off-by line at the end of the commit message
    #[arg(short = 's', long)]
    pub signoff: bool,

    #[arg(long)]
    pub disable_pre: bool,
}

pub async fn execute(args: CommitArgs) {
    /* check args */
    let index = Index::load(path::index()).unwrap();
    let storage = ClientStorage::init(path::objects());
    let tracked_entries = index.tracked_entries(0);
    if tracked_entries.is_empty() && !args.allow_empty {
        panic!("fatal: no changes added to commit, use --allow-empty to override");
    }

    // run pre commit hook
    if !args.disable_pre {
        let hooks_dir = path::hooks();

        #[cfg(not(target_os = "windows"))]
        let hook_path = hooks_dir.join("pre-commit.sh");

        #[cfg(target_os = "windows")]
        let hook_path = hooks_dir.join("pre-commit.ps1");
        if hook_path.exists() {
            let hook_display = hook_path.display();
            #[cfg(not(target_os = "windows"))]
            let output = Command::new("sh")
                .arg(&hook_path)
                .current_dir(util::working_dir())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
                .unwrap_or_else(|e| panic!("Failed to execute hook {hook_display}: {e}"));

            #[cfg(target_os = "windows")]
            let output = Command::new("powershell")
                .arg("-File")
                .arg(&hook_path)
                .current_dir(util::working_dir())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
                .unwrap_or_else(|e| panic!("Failed to execute hook {hook_display}: {e}"));

            if !output.status.success() {
                panic!(
                    "Hook {} failed with exit code {}",
                    hook_display,
                    output.status.code().unwrap_or(-1)
                );
            }
        }
    }

    //Prepare commit message
    let commit_message = if args.signoff {
        // get user
        let user_name = UserConfig::get("user", None, "name")
            .await
            .unwrap_or_else(|| "unknown".to_string());
        let user_email = UserConfig::get("user", None, "email")
            .await
            .unwrap_or_else(|| "unknown".to_string());

        // get sign line
        let signoff_line = format!("Signed-off-by: {user_name} <{user_email}>");
        format!("{}\n\n{signoff_line}", args.message)
    } else {
        args.message.clone()
    };

    // check format(if needed)
    if args.conventional && !check_conventional_commits_message(&commit_message) {
        panic!("fatal: commit message does not follow conventional commits");
    }

    /* Create tree */
    let tree = create_tree(&index, &storage, "".into()).await;

    /* Create & save commit objects */
    let parents_commit_ids = get_parents_ids().await;

    // Amend commits are only supported for a single parent commit.
    if args.amend {
        if parents_commit_ids.len() > 1 {
            panic!("fatal: --amend is not supported for merge commits with multiple parents");
        }
        let parent_commit = load_object::<Commit>(&parents_commit_ids[0]).unwrap_or_else(|_| {
            panic!(
                "fatal: not a valid object name: '{}'",
                parents_commit_ids[0]
            )
        });
        let grandpa_commit_id = parent_commit.parent_commit_ids;
        let commit = Commit::from_tree_id(
            tree.id,
            grandpa_commit_id,
            &format_commit_msg(&args.message, None),
        );

        storage
            .put(&commit.id, &commit.to_data().unwrap(), commit.get_type())
            .unwrap();

        /* update HEAD */
        update_head_and_reflog(&commit.id.to_string(), &commit_message).await;
        return;
    }

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
    update_head_and_reflog(&commit.id.to_string(), &commit_message).await;
}

/// recursively create tree from index's tracked entries
pub async fn create_tree(index: &Index, storage: &ClientStorage, current_root: PathBuf) -> Tree {
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
            Tree::from_bytes(&[], SHA1::from_str(EMPTY_TREE_HASH).unwrap()).unwrap()
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
async fn update_head<C: ConnectionTrait>(db: &C, commit_id: &str) {
    // let head = reference::Model::current_head(db).await.unwrap();
    match Head::current_with_conn(db).await {
        Head::Branch(name) => {
            // in branch
            Branch::update_branch_with_conn(db, &name, commit_id, None).await;
        }
        // None => {
        Head::Detached(_) => {
            let head = Head::Detached(SHA1::from_str(commit_id).unwrap());
            Head::update_with_conn(db, head, None).await;
        }
    }
}

async fn update_head_and_reflog(commit_id: &str, commit_message: &str) {
    let reflog_context = new_reflog_context(commit_id, commit_message).await;
    let commit_id = commit_id.to_string();
    with_reflog(
        reflog_context,
        |txn| {
            Box::pin(async move {
                update_head(txn, &commit_id).await;
                Ok(())
            })
        },
        true,
    )
    .await
    .unwrap();
}

async fn new_reflog_context(commit_id: &str, message: &str) -> ReflogContext {
    let old_oid = Head::current_commit()
        .await
        .unwrap_or(SHA1::from_bytes(&[0; 20]))
        ._to_string();
    let new_oid = commit_id.to_string();
    let action = ReflogAction::Commit {
        message: message.to_string(),
    };
    ReflogContext {
        old_oid,
        new_oid,
        action,
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use mercury::internal::object::ObjectTrait;
    use serial_test::serial;
    use tempfile::tempdir;

    use crate::utils::test::*;

    use super::*;

    #[test]
    ///Testing basic parameter parsing functionality.
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

        let args = CommitArgs::try_parse_from(["commit", "-m", "init", "--amend"]);
        assert!(args.is_ok());

        let args = CommitArgs::try_parse_from(["commit", "-m", "init", "--allow-empty", "--amend"]);
        assert!(args.is_ok());

        let args = CommitArgs::try_parse_from(["commit", "-m", "init", "-s"]);
        assert!(args.is_ok());
        assert!(args.unwrap().signoff);

        let args = CommitArgs::try_parse_from(["commit", "-m", "init", "--signoff"]);
        assert!(args.is_ok());
        assert!(args.unwrap().signoff);

        let args = CommitArgs::try_parse_from(["commit", "-m", "init", "--amend", "--signoff"]);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(args.amend);
        assert!(args.signoff);
    }

    #[tokio::test]
    #[serial]
    /// Tests the recursive tree creation from index entries.
    /// Verifies that tree objects are correctly created, saved to storage, and properly organized in a hierarchical structure.
    async fn test_create_tree() {
        let temp_path = tempdir().unwrap();
        setup_with_new_libra_in(temp_path.path()).await;
        let _guard = ChangeDirGuard::new(temp_path.path());

        let crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let index = Index::from_file(crate_path.join("../tests/data/index/index-760")).unwrap();
        println!("{:?}", index.tracked_entries(0).len());
        let storage = ClientStorage::init(path::objects());
        let tree = create_tree(&index, &storage, temp_path.keep()).await;

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
}
