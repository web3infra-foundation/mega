use crate::command::{load_object, save_object};
use crate::internal::branch::Branch;
use crate::internal::head::Head;
use crate::utils::object_ext::{BlobExt, TreeExt};
use crate::utils::{path, util};
use clap::Parser;
use common::utils::format_commit_msg;
use mercury::hash::SHA1;
use mercury::internal::index::{Index, IndexEntry};
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::{Tree, TreeItemMode};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Arguments for the revert command.
/// Reverts the specified commit by creating a new commit that undoes the changes.
#[derive(Parser, Debug)]
pub struct RevertArgs {
    /// Commit to revert (can be commit hash, branch name, or HEAD)
    #[clap(required = true)]
    pub commit: String,

    /// Don't automatically commit the revert, just stage the changes
    #[clap(short = 'n', long)]
    pub no_commit: bool,
}

/// Execute the revert command
/// This function reverts a specified commit by applying the inverse changes
/// and creating a new commit that undoes the original commit
pub async fn execute(args: RevertArgs) {
    // Check if we're in a valid repository
    if !util::check_repo_exist() {
        return;
    }

    // Ensure we're on a branch, not in detached HEAD state
    // Todo: For now, we do not handle the case when the repository is in a detached HEAD state.
    let current_head = Head::current().await;
    if let Head::Detached(_) = current_head {
        eprintln!("fatal: You are in a 'detached HEAD' state.");
        eprintln!("Reverting is not allowed in this state as it does not update any branch.");
        return;
    }

    // Resolve the commit reference to a SHA1 hash
    let commit_id = match resolve_commit(&args.commit).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("fatal: {e}");
            return;
        }
    };

    // Perform the actual revert operation
    match revert_single_commit(&commit_id, &args).await {
        Ok(revert_commit_id) => {
            if let Some(id) = revert_commit_id {
                println!(
                    "[{}] Revert commit {}",
                    &id.to_string()[..7],
                    &commit_id.to_string()[..7],
                );
            } else {
                println!("Changes staged for revert. Use 'libra commit' to finalize.");
            }
        }
        Err(e) => {
            eprintln!("error: on conflict: {e}");
            eprintln!("error: could not revert {}", &commit_id.to_string()[..7]);
        }
    }
}

/// Revert a single commit by applying its parent's state
/// This function handles the core logic of reverting a commit
async fn revert_single_commit(commit_id: &SHA1, args: &RevertArgs) -> Result<Option<SHA1>, String> {
    // Load the commit object to be reverted
    let reverted_commit: Commit =
        load_object(commit_id).map_err(|e| format!("failed to load commit: {e}"))?;

    if reverted_commit.parent_commit_ids.len() > 1 {
        return Err("Reverting merge commits is not yet supported.".to_string());
    }

    let parent_commit_id = if let Some(id) = reverted_commit.parent_commit_ids.first() {
        *id
    } else {
        return revert_root_commit(args).await;
    };

    let parent_commit: Commit =
        load_object(&parent_commit_id).map_err(|e| format!("failed to load parent commit: {e}"))?;

    // Get the current HEAD commit to apply the revert patch
    // We need to apply the reverse changes to the current state, not just restore parent state
    let current_head_commit_id = Head::current_commit()
        .await
        .ok_or("Could not get current HEAD commit")?;
    let current_commit: Commit = load_object(&current_head_commit_id).map_err(|e| e.to_string())?;

    let current_tree: Tree = load_object(&current_commit.tree_id).map_err(|e| e.to_string())?;
    let reverted_tree: Tree = load_object(&reverted_commit.tree_id).map_err(|e| e.to_string())?;
    let parent_tree: Tree = load_object(&parent_commit.tree_id).map_err(|e| e.to_string())?;

    // Convert trees to hash maps for easier manipulation
    // current_files: the state we want to modify (HEAD)
    // reverted_files: the commit we want to undo
    // parent_files: the state before the commit we're reverting
    let mut current_files: std::collections::HashMap<_, _> =
        current_tree.get_plain_items().into_iter().collect();
    let reverted_files: std::collections::HashMap<_, _> =
        reverted_tree.get_plain_items().into_iter().collect();
    let parent_files: std::collections::HashMap<_, _> =
        parent_tree.get_plain_items().into_iter().collect();

    // Apply reverse patch: for each file changed in the reverted commit,
    // undo that change in the current state
    for (path, &reverted_hash) in &reverted_files {
        let parent_hash = parent_files.get(path);

        if Some(&reverted_hash) == parent_hash {
            continue; // File unchanged in the reverted commit, skip
        }

        // Check for conflicts: if current file state differs from reverted commit state,
        // it means there were modifications after the commit we're reverting.
        // This is a simplified conflict detection.
        // TODO: This is a simplified version.
        // Conflict resolution and merge handling are intentionally omitted for now.
        if current_files.get(path) != Some(&reverted_hash) && current_files.contains_key(path) {
            return Err(format!(
                "conflict: file '{}' was modified in a later commit",
                path.display()
            ));
        }

        if let Some(parent_hash) = parent_hash {
            // File was modified or deleted -> restore to parent version
            current_files.insert(path.clone(), *parent_hash);
        } else {
            // File was newly added -> remove it
            current_files.remove(path);
        }
    }
    // Handle files that were deleted in the reverted commit
    for (path, &parent_hash) in &parent_files {
        if !reverted_files.contains_key(path) {
            current_files.insert(path.clone(), parent_hash);
        }
    }

    // Build new tree and index from the final file list
    // Note: This requires a helper function to build Tree from HashMap<PathBuf, SHA1>
    // This is a complex operation, simplified here by rebuilding the index directly
    let final_tree_id = build_tree_from_map(current_files).await?;
    let final_tree: Tree = load_object(&final_tree_id).map_err(|e| e.to_string())?;

    let mut new_index = Index::new();
    rebuild_index_from_tree(&final_tree, &mut new_index, "")?;
    let current_index = Index::load(path::index()).unwrap_or_else(|_| Index::new());
    reset_workdir_safely(&current_index, &new_index)?;
    new_index.save(path::index()).map_err(|e| e.to_string())?;

    if args.no_commit {
        Ok(None)
    } else {
        let revert_commit_id =
            create_revert_commit(commit_id, &current_head_commit_id, &final_tree_id).await?;
        Ok(Some(revert_commit_id))
    }
}

/// Helper function: Build a Tree object from a file mapping
/// This is a simplified implementation that creates a temporary index and builds a tree from it
async fn build_tree_from_map(
    files: std::collections::HashMap<PathBuf, SHA1>,
) -> Result<SHA1, String> {
    // Helper function to recursively build subtrees
    fn build_subtree(
        paths: &std::collections::HashMap<PathBuf, SHA1>,
        current_dir: &PathBuf,
    ) -> Result<Tree, String> {
        let mut tree_items = Vec::new();
        let mut subdirs = std::collections::HashMap::new();
        for (path, hash) in paths {
            if let Ok(relative_path) = path.strip_prefix(current_dir) {
                if relative_path.components().count() == 1 {
                    // File directly in the current directory
                    tree_items.push(mercury::internal::object::tree::TreeItem {
                        mode: mercury::internal::object::tree::TreeItemMode::Blob,
                        name: relative_path.to_str().unwrap().to_string(),
                        id: *hash,
                    });
                } else {
                    // File in a subdirectory
                    let subdir = current_dir.join(relative_path.components().next().unwrap());
                    subdirs
                        .entry(subdir)
                        .or_insert_with(Vec::new)
                        .push((path.clone(), *hash));
                }
            }
        }
        for (subdir, subdir_files) in subdirs {
            let subdir_tree = build_subtree(&subdir_files.into_iter().collect(), &subdir)?;
            tree_items.push(mercury::internal::object::tree::TreeItem {
                mode: mercury::internal::object::tree::TreeItemMode::Tree,
                name: subdir.file_name().unwrap().to_str().unwrap().to_string(),
                id: subdir_tree.id,
            });
        }
        Tree::from_tree_items(tree_items).map_err(|e| e.to_string())
    }
    // Start building the tree from the root directory
    let root_dir = PathBuf::new();
    let root_tree = build_subtree(&files, &root_dir)?;
    save_object(&root_tree, &root_tree.id).map_err(|e| e.to_string())?;
    Ok(root_tree.id)
}

/// Handle reverting the root commit (initial commit)
/// Root commits have no parents, so reverting them means creating an empty repository state
async fn revert_root_commit(args: &RevertArgs) -> Result<Option<SHA1>, String> {
    let new_index = Index::new(); // Create an empty index

    let current_index = Index::load(path::index()).unwrap_or_else(|_| Index::new());
    reset_workdir_safely(&current_index, &new_index)?;

    new_index
        .save(path::index())
        .map_err(|e| format!("failed to save index: {e}"))?;

    if args.no_commit {
        Ok(None)
    } else {
        // Create a commit that represents the empty repository state
        let current_head = Head::current_commit()
            .await
            .ok_or("failed to resolve current HEAD")?;
        let revert_commit_id = create_empty_revert_commit(&current_head).await?;
        Ok(Some(revert_commit_id))
    }
}

/// Rebuild the index from a tree object
/// This function recursively traverses the tree and adds all files to the index
fn rebuild_index_from_tree(tree: &Tree, index: &mut Index, prefix: &str) -> Result<(), String> {
    for item in &tree.tree_items {
        let full_path = if prefix.is_empty() {
            PathBuf::from(&item.name)
        } else {
            PathBuf::from(prefix).join(&item.name)
        };

        if let TreeItemMode::Tree = item.mode {
            // Recursively handle subdirectories
            let subtree: Tree =
                load_object(&item.id).map_err(|e| format!("failed to load subtree: {e}"))?;
            let full_path_str = full_path
                .to_str()
                .ok_or_else(|| format!("failed to convert path to UTF-8: {full_path:?}"))?;
            rebuild_index_from_tree(&subtree, index, full_path_str)?;
        } else {
            let blob = mercury::internal::object::blob::Blob::load(&item.id);
            let entry = IndexEntry::new_from_blob(
                full_path
                    .to_str()
                    .ok_or_else(|| format!("failed to convert path to UTF-8: {full_path:?}"))?
                    .to_string(),
                item.id,
                blob.data.len() as u32,
            );
            index.add(entry);
        }
    }
    Ok(())
}

/// Safely resets the working directory to match the new index state.
/// This function does NOT touch untracked files to avoid data loss.
fn reset_workdir_safely(current_index: &Index, new_index: &Index) -> Result<(), String> {
    let workdir = util::working_dir();

    let new_tracked_paths: HashSet<_> = new_index.tracked_files().into_iter().collect();

    // Step 1: Clean up - Remove files that are in the current index but not in the new one
    for path_buf in current_index.tracked_files() {
        if !new_tracked_paths.contains(&path_buf) {
            let full_path = workdir.join(path_buf);
            if full_path.exists() {
                fs::remove_file(&full_path).map_err(|e| e.to_string())?;
            }
        }
    }

    // Step 2: Restore - Create/update files that are in the new index
    for path_buf in new_index.tracked_files() {
        let path_str = path_buf.to_str().unwrap();
        if let Some(entry) = new_index.get(path_str, 0) {
            let blob = mercury::internal::object::blob::Blob::load(&entry.hash);
            let target_path = workdir.join(path_str);

            // Create parent directories if they don't exist
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&target_path, &blob.data).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// Create a revert commit that undoes the changes of the specified commit
/// The new commit will have a tree that represents the reverted state
async fn create_revert_commit(
    reverted_commit_id: &SHA1,
    parent_id: &SHA1,
    tree_id: &SHA1,
) -> Result<SHA1, String> {
    let reverted_commit: Commit = load_object(reverted_commit_id)
        .map_err(|e| format!("failed to load reverted commit: {e}"))?;

    // Create a descriptive commit message
    let revert_message = format!(
        "Revert \"{}\"\n\nThis reverts commit {}.",
        reverted_commit.message.lines().next().unwrap_or(""),
        reverted_commit_id
    );

    // Create the revert commit with the calculated tree
    let commit = Commit::from_tree_id(
        *tree_id,
        vec![*parent_id],
        &format_commit_msg(&revert_message, None),
    );

    // Save the commit object and update HEAD
    save_object(&commit, &commit.id).map_err(|e| format!("failed to save commit: {e}"))?;
    update_head(&commit.id.to_string()).await;
    Ok(commit.id)
}

/// Create a commit that reverts the root commit (creates empty repository state)
async fn create_empty_revert_commit(parent_id: &SHA1) -> Result<SHA1, String> {
    // Create an empty tree for the revert commit
    let empty_tree = create_empty_tree()?;
    let revert_message = "Revert root commit\n\nThis reverts the initial commit.";

    // Create commit with empty tree
    let commit = Commit::from_tree_id(
        empty_tree.id,
        vec![*parent_id],
        &format_commit_msg(revert_message, None),
    );

    // Save commit and update HEAD
    save_object(&commit, &commit.id).map_err(|e| format!("failed to save commit: {e}"))?;
    update_head(&commit.id.to_string()).await;
    Ok(commit.id)
}

/// Create an empty tree object (used for root commit reverts)
fn create_empty_tree() -> Result<Tree, String> {
    let tree = Tree::from_tree_items(Vec::new())
        .map_err(|e| format!("failed to create empty tree: {e}"))?;
    save_object(&tree, &tree.id).map_err(|e| e.to_string())?;
    Ok(tree)
}

/// Resolve a commit reference (hash, branch name, or HEAD) to a SHA1
async fn resolve_commit(reference: &str) -> Result<SHA1, String> {
    util::get_commit_base(reference).await
}

/// Update the HEAD reference to point to the new commit
async fn update_head(commit_id: &str) {
    if let Head::Branch(name) = Head::current().await {
        Branch::update_branch(&name, commit_id, None).await;
    }
}
