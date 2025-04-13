use std::collections::HashSet;
use std::path::PathBuf;

use colored::Colorize;

use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::Tree;

use crate::command::calc_file_blob_hash;
use crate::internal::head::Head;
use crate::utils::object_ext::{CommitExt, TreeExt};
use crate::utils::{path, util};
use mercury::internal::index::Index;

/// path: to workdir
#[derive(Debug, Default, Clone)]
pub struct Changes {
    pub new: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub deleted: Vec<PathBuf>,
}
impl Changes {
    pub fn is_empty(&self) -> bool {
        self.new.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }

    /// to relative path(to cur_dir)
    pub fn to_relative(&self) -> Changes {
        let mut change = self.clone();
        [&mut change.new, &mut change.modified, &mut change.deleted]
            .into_iter()
            .for_each(|paths| {
                *paths = paths.iter().map(util::workdir_to_current).collect();
            });
        change
    }
}

/**
 * 2 parts:
 * 1. unstaged
 * 2. staged to be committed
 */
pub async fn execute() {
    if !util::check_repo_exist() {
        return;
    }
    match Head::current().await {
        Head::Detached(commit) => {
            println!(
                "HEAD detached at {}",
                String::from_utf8_lossy(&commit.0[0..7])
            );
        }
        Head::Branch(branch) => {
            println!("On branch {}", branch);
        }
    }

    if Head::current_commit().await.is_none() {
        println!("\nNo commits yet\n");
    }

    // to cur_dir relative path
    let staged = changes_to_be_committed().await.to_relative();
    let unstaged = changes_to_be_staged().to_relative();
    if staged.is_empty() && unstaged.is_empty() {
        println!("nothing to commit, working tree clean");
        return;
    }

    if !staged.is_empty() {
        println!("Changes to be committed:");
        println!("  use \"libra restore --staged <file>...\" to unstage");
        staged.deleted.iter().for_each(|f| {
            let str = format!("\tdeleted: {}", f.display());
            println!("{}", str.bright_green());
        });
        staged.modified.iter().for_each(|f| {
            let str = format!("\tmodified: {}", f.display());
            println!("{}", str.bright_green());
        });
        staged.new.iter().for_each(|f| {
            let str = format!("\tnew file: {}", f.display());
            println!("{}", str.bright_green());
        });
    }

    if !unstaged.deleted.is_empty() || !unstaged.modified.is_empty() {
        println!("Changes not staged for commit:");
        println!("  use \"libra add <file>...\" to update what will be committed");
        println!("  use \"libra restore <file>...\" to discard changes in working directory");
        unstaged.deleted.iter().for_each(|f| {
            let str = format!("\tdeleted: {}", f.display());
            println!("{}", str.bright_red());
        });
        unstaged.modified.iter().for_each(|f| {
            let str = format!("\tmodified: {}", f.display());
            println!("{}", str.bright_red());
        });
    }
    if !unstaged.new.is_empty() {
        println!("Untracked files:");
        println!("  use \"libra add <file>...\" to include in what will be committed");
        unstaged.new.iter().for_each(|f| {
            let str = format!("\t{}", f.display());
            println!("{}", str.bright_red());
        });
    }
}

/// Check if the working tree is clean
pub async fn is_clean() -> bool {
    let staged = changes_to_be_committed().await;
    let unstaged = changes_to_be_staged();
    staged.is_empty() && unstaged.is_empty()
}

/**
 * Compare the difference between `index` and the last `Commit Tree`
 */
pub async fn changes_to_be_committed() -> Changes {
    let mut changes = Changes::default();
    let index = Index::load(path::index()).unwrap();
    let head_commit = Head::current_commit().await;
    let tracked_files = index.tracked_files();

    if head_commit.is_none() {
        // no commit yet
        changes.new = tracked_files;
        return changes;
    }

    let head_commit = head_commit.unwrap();
    let commit = Commit::load(&head_commit);
    let tree = Tree::load(&commit.tree_id);
    let tree_files = tree.get_plain_items();

    for (item_path, item_hash) in tree_files.iter() {
        let item_str = item_path.to_str().unwrap();
        if index.tracked(item_str, 0) {
            if !index.verify_hash(item_str, 0, item_hash) {
                changes.modified.push(item_path.clone());
            }
        } else {
            // in the last commit but not in the index
            changes.deleted.push(item_path.clone());
        }
    }
    let tree_files_set: HashSet<PathBuf> = tree_files.into_iter().map(|(path, _)| path).collect();
    // `new` means the files in index but not in the last commit
    changes.new = tracked_files
        .into_iter()
        .filter(|path| !tree_files_set.contains(path))
        .collect();

    changes
}

/// Compare the difference between `index` and the `workdir`
pub fn changes_to_be_staged() -> Changes {
    let mut changes = Changes::default();
    let workdir = util::working_dir();
    let index = Index::load(path::index()).unwrap();
    let tracked_files = index.tracked_files();
    for file in tracked_files.iter() {
        let file_str = file.to_str().unwrap();
        let file_abs = util::workdir_to_absolute(file);
        if util::check_gitignore(&workdir, &file_abs) {
            continue;
        } else if !file_abs.exists() {
            changes.deleted.push(file.clone());
        } else if index.is_modified(file_str, 0, &workdir) {
            // only calc the hash if the file is modified (metadata), for optimization
            let file_hash = calc_file_blob_hash(&file_abs).unwrap();
            if !index.verify_hash(file_str, 0, &file_hash) {
                changes.modified.push(file.clone());
            }
        }
    }
    let files = util::list_workdir_files().unwrap(); // to workdir
    for file in files.iter() {
        let file_abs = util::workdir_to_absolute(file);
        if util::check_gitignore(&workdir, &file_abs) {
            // file ignored in .libraignore
            continue;
        }
        if !index.tracked(file.to_str().unwrap(), 0) {
            // file not tracked in `index`
            changes.new.push(file.clone());
        }
    }
    changes
}

#[cfg(test)]
mod test {
    use std::{fs, io::Write, path::Path};

    use super::*;
    use crate::{
        command::{self, add::AddArgs},
        utils::test::{self, TEST_DIR},
    };

    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_changes_to_be_staged() {
        let test_dir = Path::new(TEST_DIR);
        if test_dir.exists() {
            fs::remove_dir_all(test_dir).unwrap();
        }

        test::setup_with_new_libra().await;

        let mut gitignore_file = fs::File::create(".libraignore").unwrap();
        gitignore_file
            .write_all(b"should_ignore*\nignore_dir/")
            .unwrap();

        let mut should_ignore_file_0 = fs::File::create("should_ignore.0").unwrap();
        let mut not_ignore_file_0 = fs::File::create("not_ignore.0").unwrap();
        fs::create_dir("ignore_dir").unwrap();
        let mut should_ignore_file_1 = fs::File::create("ignore_dir/should_ignore.1").unwrap();
        fs::create_dir("not_ignore_dir").unwrap();
        let mut not_ignore_file_1 = fs::File::create("not_ignore_dir/not_ignore.1").unwrap();

        let change = changes_to_be_staged();
        assert!(!change
            .new
            .iter()
            .any(|x| x.file_name().unwrap() == "should_ignore.0"));
        assert!(!change
            .new
            .iter()
            .any(|x| x.file_name().unwrap() == "should_ignore.1"));
        assert!(change
            .new
            .iter()
            .any(|x| x.file_name().unwrap() == "not_ignore.0"));
        assert!(change
            .new
            .iter()
            .any(|x| x.file_name().unwrap() == "not_ignore.1"));

        command::add::execute(AddArgs {
            pathspec: vec![String::from(".")],
            all: true,
            update: false,
            verbose: false,
        })
        .await;

        should_ignore_file_0.write_all(b"foo").unwrap();
        should_ignore_file_1.write_all(b"foo").unwrap();
        not_ignore_file_0.write_all(b"foo").unwrap();
        not_ignore_file_1.write_all(b"foo").unwrap();

        let change = changes_to_be_staged();
        assert!(!change
            .modified
            .iter()
            .any(|x| x.file_name().unwrap() == "should_ignore.0"));
        assert!(!change
            .modified
            .iter()
            .any(|x| x.file_name().unwrap() == "should_ignore.1"));
        assert!(change
            .modified
            .iter()
            .any(|x| x.file_name().unwrap() == "not_ignore.0"));
        assert!(change
            .modified
            .iter()
            .any(|x| x.file_name().unwrap() == "not_ignore.1"));

        fs::remove_dir_all("ignore_dir").unwrap();
        fs::remove_dir_all("not_ignore_dir").unwrap();
        fs::remove_file("should_ignore.0").unwrap();
        fs::remove_file("not_ignore.0").unwrap();

        not_ignore_file_1.write_all(b"foo").unwrap();

        let change = changes_to_be_staged();
        assert!(!change
            .deleted
            .iter()
            .any(|x| x.file_name().unwrap() == "should_ignore.0"));
        assert!(!change
            .deleted
            .iter()
            .any(|x| x.file_name().unwrap() == "should_ignore.1"));
        assert!(change
            .deleted
            .iter()
            .any(|x| x.file_name().unwrap() == "not_ignore.0"));
        assert!(change
            .deleted
            .iter()
            .any(|x| x.file_name().unwrap() == "not_ignore.1"));
    }
}
