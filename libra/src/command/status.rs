use std::collections::HashSet;
use std::path::PathBuf;

use colored::Colorize;

use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::Tree;

use crate::command::calc_file_blob_hash;
use crate::internal::head::Head;
use crate::utils::object_ext::{CommitExt, TreeExt};
use crate::utils::{path, util};
use clap::Parser;
use mercury::internal::index::Index;
use std::io::Write;

#[derive(Parser, Debug, Default)]
pub struct StatusArgs {
    /// Output in a machine-readable format
    #[clap(long = "porcelain")]
    pub porcelain: bool,

    /// Give the output in the short-format
    #[clap(short = 's', long = "short")]
    pub short: bool,
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
pub async fn execute_to(args: StatusArgs, writer: &mut impl Write) {
    if !util::check_repo_exist() {
        return;
    }

    // Do not output branch info in porcelain or short mode
    if !args.porcelain && !args.short {
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

    // Use short format output
    if args.short {
        output_short_format(&staged, &unstaged, writer).await;
        return;
    }

    if staged.is_empty() && unstaged.is_empty() {
        writeln!(writer, "nothing to commit, working tree clean").unwrap();
        return;
    }

    if !staged.is_empty() {
        println!("Changes to be committed:");
        println!("  use \"libra restore --staged <file>...\" to unstage");
        staged.deleted.iter().for_each(|f| {
            let str = format!("\tdeleted: {}", f.display());
            writeln!(writer, "{}", str.bright_green()).unwrap();
        });
        staged.modified.iter().for_each(|f| {
            let str = format!("\tmodified: {}", f.display());
            writeln!(writer, "{}", str.bright_green()).unwrap();
        });
        staged.new.iter().for_each(|f| {
            let str = format!("\tnew file: {}", f.display());
            writeln!(writer, "{}", str.bright_green()).unwrap();
        });
    }

    if !unstaged.deleted.is_empty() || !unstaged.modified.is_empty() {
        println!("Changes not staged for commit:");
        println!("  use \"libra add <file>...\" to update what will be committed");
        println!("  use \"libra restore <file>...\" to discard changes in working directory");
        unstaged.deleted.iter().for_each(|f| {
            let str = format!("\tdeleted: {}", f.display());
            writeln!(writer, "{}", str.bright_red()).unwrap();
        });
        unstaged.modified.iter().for_each(|f| {
            let str = format!("\tmodified: {}", f.display());
            writeln!(writer, "{}", str.bright_red()).unwrap();
        });
    }
    if !unstaged.new.is_empty() {
        println!("Untracked files:");
        println!("  use \"libra add <file>...\" to include in what will be committed");
        unstaged.new.iter().for_each(|f| {
            let str = format!("\t{}", f.display());
            writeln!(writer, "{}", str.bright_red()).unwrap();
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

/// Core logic for generating short format status without color (for testing)
pub fn generate_short_format_status(
    staged: &Changes,
    unstaged: &Changes,
) -> Vec<(std::path::PathBuf, char, char)> {
    use std::collections::HashMap;

    // Create a map to track all files and their status
    let mut file_status: HashMap<PathBuf, (char, char)> = HashMap::new();

    // Process staged changes
    for file in &staged.new {
        file_status.insert(file.clone(), ('A', ' '));
    }
    for file in &staged.modified {
        file_status.insert(file.clone(), ('M', ' '));
    }
    for file in &staged.deleted {
        file_status.insert(file.clone(), ('D', ' '));
    }

    // Helper to process unstaged changes (modified/deleted)
    fn process_unstaged_changes(
        files: &[PathBuf],
        file_status: &mut std::collections::HashMap<PathBuf, (char, char)>,
        unstaged_char: char,
    ) {
        for file in files {
            let staged_status = file_status.get(file).map(|(s, _)| *s);
            if let Some(status) = staged_status {
                // File is both staged and unstaged - keep staged status, update unstaged
                file_status.insert(file.clone(), (status, unstaged_char));
            } else {
                // File is only unstaged
                file_status.insert(file.clone(), (' ', unstaged_char));
            }
        }
    }

    // Process unstaged changes
    process_unstaged_changes(&unstaged.modified, &mut file_status, 'M');
    process_unstaged_changes(&unstaged.deleted, &mut file_status, 'D');

    for file in &unstaged.new {
        // Untracked files
        file_status.insert(file.clone(), ('?', '?'));
    }

    // Sort files by path for consistent output
    let mut sorted_files: Vec<_> = file_status.iter().collect();
    sorted_files.sort_by(|a, b| a.0.cmp(b.0));

    sorted_files
        .into_iter()
        .map(|(file, (staged_status, unstaged_status))| {
            (file.clone(), *staged_status, *unstaged_status)
        })
        .collect()
}

pub async fn output_short_format(staged: &Changes, unstaged: &Changes, writer: &mut impl Write) {
    // Check if colors should be used
    let use_colors = should_use_colors().await;

    // Get the status information using the core logic
    let status_list = generate_short_format_status(staged, unstaged);

    // Output the short format
    for (file, staged_status, unstaged_status) in status_list {
        if use_colors {
            let colored_output = format_colored_status(staged_status, unstaged_status, &file);
            writeln!(writer, "{}", colored_output).unwrap();
        } else {
            writeln!(
                writer,
                "{}{} {}",
                staged_status,
                unstaged_status,
                file.display()
            )
            .unwrap();
        }
    }
}

/// Check if colors should be used based on configuration
async fn should_use_colors() -> bool {
    use crate::internal::config::Config;
    use std::io::{self, IsTerminal};

    // Check color.status.short configuration
    if let Some(color_setting) = Config::get("color", Some("status"), "short").await {
        match color_setting.as_str() {
            "always" => true,
            "never" | "false" => false,
            "auto" | "true" => {
                // Check if output is to a terminal
                io::stdout().is_terminal()
            }
            _ => false,
        }
    } else {
        // Check color.ui configuration as fallback
        if let Some(color_setting) = Config::get("color", None, "ui").await {
            match color_setting.as_str() {
                "always" => true,
                "never" | "false" => false,
                "auto" | "true" => {
                    // Check if output is to a terminal
                    io::stdout().is_terminal()
                }
                _ => false,
            }
        } else {
            // Default to auto (check if terminal)
            io::stdout().is_terminal()
        }
    }
}

/// Format the status with colors according to Git conventions
fn format_colored_status(
    staged_status: char,
    unstaged_status: char,
    file: &std::path::Path,
) -> String {
    use colored::Colorize;

    // Color the status characters based on Git conventions
    let colored_staged = match staged_status {
        'A' => staged_status.to_string().green(),
        'M' => staged_status.to_string().green(),
        'D' => staged_status.to_string().red(),
        'R' => staged_status.to_string().yellow(),
        'C' => staged_status.to_string().yellow(),
        'U' => staged_status.to_string().red(),
        '?' => staged_status.to_string().bright_red(),
        ' ' => staged_status.to_string().into(),
        _ => staged_status.to_string().into(),
    };

    let colored_unstaged = match unstaged_status {
        'M' => unstaged_status.to_string().red(),
        'D' => unstaged_status.to_string().red(),
        'U' => unstaged_status.to_string().red(),
        '?' => unstaged_status.to_string().bright_red(),
        '!' => unstaged_status.to_string().bright_red(),
        ' ' => unstaged_status.to_string().into(),
        _ => unstaged_status.to_string().into(),
    };

    format!("{}{} {}", colored_staged, colored_unstaged, file.display())
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
