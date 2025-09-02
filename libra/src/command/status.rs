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
use std::io::Write;
use clap::Parser;

#[derive(Parser, Debug, Default)]
pub struct StatusArgs {
    /// Output in a machine-readable format
    #[clap(long = "porcelain")]
    pub porcelain: bool,
}

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
 pub async fn execute_to(args: StatusArgs,writer: &mut impl Write) {
    if !util::check_repo_exist() {
        return;
    }
    
    // Do not output branch info in porcelain mode
    if !args.porcelain {
        match Head::current().await {
            Head::Detached(commit_hash) => {
                writeln!(writer, "HEAD detached at {}", &commit_hash.to_string()[..8]).unwrap();
            }
            Head::Branch(branch) => {
                writeln!(writer, "On branch {branch}").unwrap();
            }
        }

        if Head::current_commit().await.is_none() {
            writeln!(writer, "\nNo commits yet\n").unwrap();
        }
    }    
    
    // to cur_dir relative path
    let staged = changes_to_be_committed().await.to_relative();
    let unstaged = changes_to_be_staged().to_relative();

    // Use machine-readable output in porcelain mode
    if args.porcelain {
        output_porcelain(&staged, &unstaged, writer);
        return;
    }


    if staged.is_empty() && unstaged.is_empty() {
        writeln!(writer,"nothing to commit, working tree clean").unwrap();
        return;
    }

    if !staged.is_empty() {
        println!("Changes to be committed:");
        println!("  use \"libra restore --staged <file>...\" to unstage");
        staged.deleted.iter().for_each(|f| {
            let str = format!("\tdeleted: {}", f.display());
            writeln!(writer,"{}", str.bright_green()).unwrap();
        });
        staged.modified.iter().for_each(|f| {
            let str = format!("\tmodified: {}", f.display());
            writeln!(writer,"{}", str.bright_green()).unwrap();
        });
        staged.new.iter().for_each(|f| {
            let str = format!("\tnew file: {}", f.display());
            writeln!(writer,"{}", str.bright_green()).unwrap();
        });
    }

    if !unstaged.deleted.is_empty() || !unstaged.modified.is_empty() {
        println!("Changes not staged for commit:");
        println!("  use \"libra add <file>...\" to update what will be committed");
        println!("  use \"libra restore <file>...\" to discard changes in working directory");
        unstaged.deleted.iter().for_each(|f| {
            let str = format!("\tdeleted: {}", f.display());
            writeln!(writer,"{}", str.bright_red()).unwrap();
        });
        unstaged.modified.iter().for_each(|f| {
            let str = format!("\tmodified: {}", f.display());
            writeln!(writer,"{}", str.bright_red()).unwrap();
        });
    }
    if !unstaged.new.is_empty() {
        println!("Untracked files:");
        println!("  use \"libra add <file>...\" to include in what will be committed");
        unstaged.new.iter().for_each(|f| {
            let str = format!("\t{}", f.display());
            writeln!(writer,"{}", str.bright_red()).unwrap();
        });
    }
}

pub fn output_porcelain(staged: &Changes, unstaged: &Changes, writer: &mut impl Write) {
    // Output changes in the staging area
    for file in &staged.new {
        writeln!(writer, "A  {}", file.display()).unwrap();
    }
    for file in &staged.modified {
        writeln!(writer, "M  {}", file.display()).unwrap();
    }
    for file in &staged.deleted {
        writeln!(writer, "D  {}", file.display()).unwrap();
    }
    
    // Output unstaged changes
    for file in &unstaged.modified {
        writeln!(writer, " M {}", file.display()).unwrap();
    }
    for file in &unstaged.deleted {
        writeln!(writer, " D {}", file.display()).unwrap();
    }
    
    // Output untracked files
    for file in &unstaged.new {
        writeln!(writer, "?? {}", file.display()).unwrap();
    }

}

pub async fn execute(args: StatusArgs) {
    execute_to(args, &mut std::io::stdout()).await
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
mod test {}
