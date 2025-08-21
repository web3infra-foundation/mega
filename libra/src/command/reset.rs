use crate::command::{get_target_commit, load_object};
use crate::internal::branch::Branch;
use crate::internal::head::Head;
use crate::utils::object_ext::{BlobExt, TreeExt};
use crate::utils::{path, util};
use clap::Parser;
use mercury::hash::SHA1;
use mercury::internal::index::{Index, IndexEntry};
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::Tree;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use crate::internal::db::get_db_conn_instance;
use crate::internal::reflog::{with_reflog, ReflogAction, ReflogContext};

#[derive(Parser, Debug)]
pub struct ResetArgs {
    /// The commit to reset to (default: HEAD)
    #[clap(default_value = "HEAD")]
    pub target: String,

    /// Soft reset: only move HEAD pointer
    #[clap(long, group = "mode")]
    pub soft: bool,

    /// Mixed reset: move HEAD and reset index (default)
    #[clap(long, group = "mode")]
    pub mixed: bool,

    /// Hard reset: move HEAD, reset index and working directory
    #[clap(long, group = "mode")]
    pub hard: bool,

    /// Pathspecs to reset specific files
    #[clap(value_name = "PATH")]
    pub pathspecs: Vec<String>,
}

#[derive(Debug)]
enum ResetMode {
    Soft,
    Mixed,
    Hard,
}

/// Execute the reset command with the given arguments.
/// Resets the current HEAD to the specified state, with different modes:
/// - Soft: Only moves HEAD pointer
/// - Mixed: Moves HEAD and resets index (default)
/// - Hard: Moves HEAD, resets index and working directory
pub async fn execute(args: ResetArgs) {
    if !util::check_repo_exist() {
        return;
    }

    // Determine reset mode
    let mode = if args.soft {
        ResetMode::Soft
    } else if args.hard {
        ResetMode::Hard
    } else {
        ResetMode::Mixed // default
    };

    // Handle pathspec reset (only affects index)
    if !args.pathspecs.is_empty() {
        reset_pathspecs(&args.pathspecs, &args.target).await;
        return;
    }

    // Resolve target commit
    let target_commit_id = match resolve_commit(&args.target).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("fatal: {e}");
            return;
        }
    };

    // Perform reset based on mode
    match perform_reset(target_commit_id, mode, &args.target).await {
        Ok(_) => {
            println!(
                "HEAD is now at {} {}",
                &target_commit_id.to_string()[..7],
                get_commit_summary(&target_commit_id).unwrap_or_else(|_| "".to_string())
            );
        }
        Err(e) => {
            eprintln!("fatal: {e}");
        }
    }
}

/// Reset specific files in the index to their state in the target commit.
/// This function only affects the index, not the working directory.
async fn reset_pathspecs(pathspecs: &[String], target: &str) {
    // Reset specific files in index to target commit
    let target_commit_id = match resolve_commit(target).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("fatal: {e}");
            return;
        }
    };

    let commit: Commit = match load_object(&target_commit_id) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("fatal: failed to load commit: {e}");
            return;
        }
    };

    let tree: Tree = match load_object(&commit.tree_id) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("fatal: failed to load tree: {e}");
            return;
        }
    };

    let index_file = path::index();
    let mut index = match Index::load(&index_file) {
        Ok(idx) => idx,
        Err(e) => {
            eprintln!("fatal: failed to load index: {e}");
            return;
        }
    };
    let mut changed = false;

    for pathspec in pathspecs {
        let relative_path = util::workdir_to_current(PathBuf::from(pathspec));
        let path_str = relative_path.to_str().expect("Path contains invalid UTF-8");

        match find_tree_item(&tree, path_str) {
            Some(item) => {
                let blob: mercury::internal::object::blob::Blob = load_object(&item.id).unwrap();
                let entry = IndexEntry::new_from_blob(
                    path_str.to_string(),
                    item.id,
                    blob.data.len() as u32,
                );
                index.add(entry);
                println!("Unstaged changes after reset of: {pathspec}");
                changed = true;
            }
            None => {
                if index.get(path_str, 0).is_some() {
                    index.remove(path_str, 0);
                    println!("Removed from staging: {pathspec}");
                    changed = true;
                } else {
                    eprintln!(
                        "error: pathspec '{pathspec}' did not match any file(s) known to libra"
                    );
                }
            }
        }
    }

    if changed {
        if let Err(e) = index.save(&index_file) {
            eprintln!("fatal: failed to save index: {e}");
        }
    }
}

/// Perform the actual reset operation based on the specified mode.
/// Updates HEAD pointer and optionally resets index and working directory.
async fn perform_reset(
    target_commit_id: SHA1,
    mode: ResetMode,
    target_ref_str: &str, // e.g, "HEAD~2"
) -> Result<(), String> {
    // avoids holding the transaction open while doing read-only preparations.
    let db = get_db_conn_instance().await;
    let old_oid = Head::current_commit_with_conn(db)
        .await
        .ok_or_else(|| "Cannot reset: HEAD is unborn and points to no commit.".to_string())?;

    if old_oid == target_commit_id {
        println!("HEAD already at {}, nothing to do.", &target_commit_id.to_string()[..7]);
        return Ok(());
    }

    // determine if HEAD is attached to a branch or detached. This is crucial for
    // deciding which reference pointer to update in the transaction.
    let current_head_state = Head::current_with_conn(db).await;

    let action = ReflogAction::Reset { target: target_ref_str.to_string() };
    let context = ReflogContext {
        old_oid: old_oid.to_string(),
        new_oid: target_commit_id.to_string(),
        action,
    };

    with_reflog(
        context,
        move |txn| {
            Box::pin(async move {
                match &current_head_state {
                    // If on a branch, update the branch pointer. HEAD will move with it.
                    Head::Branch(branch_name) => {
                        Branch::update_branch_with_conn(txn, branch_name, &target_commit_id.to_string(), None).await;
                    }
                    // If in a detached state, update the HEAD pointer directly.
                    Head::Detached(_) => {
                        let new_head = Head::Detached(target_commit_id);
                        Head::update_with_conn(txn, new_head, None).await;
                    }
                }
                Ok(())
            })
        },
        true,
    )
        .await
        .map_err(|e| e.to_string())?;

    match mode {
        ResetMode::Soft => {
            // Only move HEAD, nothing else to do
        }
        ResetMode::Mixed => {
            // Reset index to target commit
            reset_index_to_commit(&target_commit_id)?;
        }
        ResetMode::Hard => {
            // Reset index and working directory
            reset_index_to_commit(&target_commit_id)?;
            reset_working_directory_to_commit(&target_commit_id, Some(old_oid)).await?;
        }
    }
    Ok(())
}

/// Reset the index to match the specified commit's tree.
/// Clears the current index and rebuilds it from the commit's tree structure.
pub(crate) fn reset_index_to_commit(commit_id: &SHA1) -> Result<(), String> {
    let commit: Commit =
        load_object(commit_id).map_err(|e| format!("failed to load commit: {e}"))?;

    let tree: Tree =
        load_object(&commit.tree_id).map_err(|e| format!("failed to load tree: {e}"))?;

    let index_file = path::index();
    let mut index = Index::new();

    // Rebuild index from tree
    rebuild_index_from_tree(&tree, &mut index, "")?;

    index
        .save(&index_file)
        .map_err(|e| format!("failed to save index: {e}"))?;

    Ok(())
}

/// Reset the working directory to match the specified commit.
/// Removes files that exist in the original commit but not in the target commit,
/// and restores files from the target commit's tree.
pub(crate) async fn reset_working_directory_to_commit(
    commit_id: &SHA1,
    original_head_commit: Option<SHA1>,
) -> Result<(), String> {
    let commit: Commit =
        load_object(commit_id).map_err(|e| format!("failed to load commit: {e}"))?;

    let tree: Tree =
        load_object(&commit.tree_id).map_err(|e| format!("failed to load tree: {e}"))?;

    let workdir = util::working_dir();

    // Use the original HEAD commit to determine what files to clean up
    if let Some(current_commit_id) = original_head_commit {
        if current_commit_id != *commit_id {
            // Remove files that exist in current commit but not in target commit
            let current_commit: Commit = load_object(&current_commit_id)
                .map_err(|e| format!("failed to load current commit: {e}"))?;
            let current_tree: Tree = load_object(&current_commit.tree_id)
                .map_err(|e| format!("failed to load current tree: {e}"))?;

            let current_files = current_tree.get_plain_items();
            let target_files: Vec<_> = tree.get_plain_items();
            let target_files_set: HashSet<_> = target_files.iter().map(|(path, _)| path).collect();

            // Remove files that are in current commit but not in target commit
            for (file_path, _) in current_files {
                if !target_files_set.contains(&file_path) {
                    let full_path = workdir.join(&file_path);
                    if full_path.exists() {
                        if let Err(e) = fs::remove_file(&full_path) {
                            eprintln!("warning: failed to remove {}: {}", full_path.display(), e);
                        }
                    }
                }
            }
        }
    } else {
        // No current HEAD, remove all tracked files from index
        let index = Index::load(path::index()).unwrap_or_else(|_| Index::new());
        let tracked_files = index.tracked_files();

        for file_path in tracked_files {
            let full_path = workdir.join(&file_path);
            if full_path.exists() {
                if let Err(e) = fs::remove_file(&full_path) {
                    eprintln!("warning: failed to remove {}: {}", full_path.display(), e);
                }
            }
        }
    }

    // Remove empty directories
    remove_empty_directories(&workdir)?;

    // Restore files from target tree
    restore_working_directory_from_tree(&tree, &workdir, "")?;

    Ok(())
}

/// Recursively rebuild the index from a tree structure.
/// Traverses the tree and adds all files to the index with their blob hashes.
pub(crate) fn rebuild_index_from_tree(tree: &Tree, index: &mut Index, prefix: &str) -> Result<(), String> {
    for item in &tree.tree_items {
        let full_path = if prefix.is_empty() {
            item.name.clone()
        } else {
            format!("{}/{}", prefix, item.name)
        };

        match item.mode {
            mercury::internal::object::tree::TreeItemMode::Tree => {
                let subtree: Tree =
                    load_object(&item.id).map_err(|e| format!("failed to load subtree: {e}"))?;
                rebuild_index_from_tree(&subtree, index, &full_path)?;
            }
            _ => {
                // Add file to index - but don't modify working directory files
                // Use the blob hash from the tree, not from working directory
                // Get blob size for IndexEntry
                let blob = mercury::internal::object::blob::Blob::load(&item.id);

                // Create IndexEntry with the tree's blob hash
                let entry = IndexEntry::new_from_blob(full_path, item.id, blob.data.len() as u32);
                index.add(entry);
            }
        }
    }
    Ok(())
}

/// Restore the working directory from a tree structure.
/// Recursively creates directories and writes files from the tree's blob objects.
pub(crate) fn restore_working_directory_from_tree(
    tree: &Tree,
    workdir: &Path,
    prefix: &str,
) -> Result<(), String> {
    for item in &tree.tree_items {
        let full_path = if prefix.is_empty() {
            item.name.clone()
        } else {
            format!("{}/{}", prefix, item.name)
        };

        let file_path = workdir.join(&full_path);

        match item.mode {
            mercury::internal::object::tree::TreeItemMode::Tree => {
                // Create directory
                fs::create_dir_all(&file_path).map_err(|e| {
                    format!("failed to create directory {}: {}", file_path.display(), e)
                })?;

                let subtree: Tree =
                    load_object(&item.id).map_err(|e| format!("failed to load subtree: {e}"))?;
                restore_working_directory_from_tree(&subtree, workdir, &full_path)?;
            }
            _ => {
                // Restore file
                let blob = load_object::<mercury::internal::object::blob::Blob>(&item.id)
                    .map_err(|e| format!("failed to load blob: {e}"))?;

                // Create parent directory if needed
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        format!("failed to create directory {}: {}", parent.display(), e)
                    })?;
                }

                fs::write(&file_path, blob.data)
                    .map_err(|e| format!("failed to write file {}: {}", file_path.display(), e))?;
            }
        }
    }
    Ok(())
}

/// Remove empty directories from the working directory.
/// Recursively traverses the directory tree and removes any empty directories,
/// except for the .libra directory and the working directory root.
pub(crate) fn remove_empty_directories(workdir: &Path) -> Result<(), String> {
    fn remove_empty_dirs_recursive(dir: &Path, workdir: &Path) -> Result<(), String> {
        if !dir.is_dir() || dir == workdir {
            return Ok(());
        }

        let entries = fs::read_dir(dir)
            .map_err(|e| format!("failed to read directory {}: {}", dir.display(), e))?;

        let mut has_files = false;
        let mut subdirs = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
            let path = entry.path();

            if path.is_dir() {
                // Don't remove .libra directory
                if path.file_name().and_then(|n| n.to_str()) == Some(".libra") {
                    has_files = true;
                } else {
                    subdirs.push(path);
                }
            } else {
                has_files = true;
            }
        }

        // Recursively process subdirectories
        for subdir in subdirs {
            remove_empty_dirs_recursive(&subdir, workdir)?;

            // Check if subdir is now empty
            if subdir
                .read_dir()
                .map(|mut d| d.next().is_none())
                .unwrap_or(false)
            {
                if let Err(e) = fs::remove_dir(&subdir) {
                    eprintln!(
                        "warning: failed to remove empty directory {}: {}",
                        subdir.display(),
                        e
                    );
                }
            } else {
                has_files = true;
            }
        }

        // Remove this directory if it's empty and not the working directory
        if !has_files && dir != workdir {
            if let Err(e) = fs::remove_dir(dir) {
                eprintln!(
                    "warning: failed to remove empty directory {}: {}",
                    dir.display(),
                    e
                );
            }
        }

        Ok(())
    }

    // Start from working directory and process all subdirectories
    let entries =
        fs::read_dir(workdir).map_err(|e| format!("failed to read working directory: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
        let path = entry.path();

        if path.is_dir() && path.file_name().and_then(|n| n.to_str()) != Some(".libra") {
            remove_empty_dirs_recursive(&path, workdir)?;
        }
    }

    Ok(())
}

/// Resolve a reference string to a commit SHA1.
/// Accepts commit hashes, branch names, or HEAD references.
async fn resolve_commit(reference: &str) -> Result<SHA1, String> {
    get_target_commit(reference)
        .await
        .map_err(|e| e.to_string())
}

/// Get the first line of a commit's message for display purposes.
fn get_commit_summary(commit_id: &SHA1) -> Result<String, String> {
    let commit: Commit =
        load_object(commit_id).map_err(|e| format!("failed to load commit: {e}"))?;

    let first_line = commit.message.lines().next().unwrap_or("").to_string();
    Ok(first_line)
}

/// Find a specific file or directory in a tree by path.
/// Returns the tree item if found, None otherwise.
fn find_tree_item(tree: &Tree, path: &str) -> Option<mercury::internal::object::tree::TreeItem> {
    let parts: Vec<&str> = path.split('/').collect();
    find_tree_item_recursive(tree, &parts, 0)
}

/// Recursively search for a tree item by path components.
/// Helper function for find_tree_item that handles nested directory structures.
fn find_tree_item_recursive(
    tree: &Tree,
    parts: &[&str],
    index: usize,
) -> Option<mercury::internal::object::tree::TreeItem> {
    if index >= parts.len() {
        return None;
    }

    for item in &tree.tree_items {
        if item.name == parts[index] {
            if index == parts.len() - 1 {
                // Found the target
                return Some(item.clone());
            } else if item.mode == mercury::internal::object::tree::TreeItemMode::Tree {
                // Continue searching in subtree
                if let Ok(subtree) = load_object::<Tree>(&item.id) {
                    if let Some(result) = find_tree_item_recursive(&subtree, parts, index + 1) {
                        return Some(result);
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_args_parse() {
        let args = ResetArgs::try_parse_from(["reset", "--hard", "HEAD~1"]).unwrap();
        assert!(args.hard);
        assert_eq!(args.target, "HEAD~1");
    }

    #[test]
    fn test_reset_mode_detection() {
        let args = ResetArgs::try_parse_from(["reset", "--soft"]).unwrap();
        assert!(args.soft);

        let args = ResetArgs::try_parse_from(["reset"]).unwrap();
        assert!(!args.soft && !args.hard);
    }
}
