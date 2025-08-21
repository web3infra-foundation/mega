use crate::command::{load_object, save_object};
use crate::internal::branch::Branch;
use crate::internal::head::Head;
use crate::utils::object_ext::{BlobExt, TreeExt};
use crate::utils::{path, util};
use clap::Parser;
use mercury::hash::SHA1;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::Tree;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use sea_orm::{TransactionTrait};
use crate::internal::db::get_db_conn_instance;
use crate::internal::reflog;
use crate::internal::reflog::{with_reflog, ReflogAction, ReflogContext, ReflogError};

/// Command-line arguments for the rebase operation
#[derive(Parser, Debug)]
pub struct RebaseArgs {
    /// The upstream branch to rebase the current branch onto.
    /// This can be a branch name, commit hash, or other Git reference.
    #[clap(required = true)]
    pub upstream: String,
}

/// Execute the rebase command
///
/// Rebase moves or combines a sequence of commits to a new base commit.
/// This implementation performs a linear rebase by:
/// 1. Finding the common ancestor between current branch and upstream
/// 2. Collecting all commits from the common ancestor to current HEAD
/// 3. Replaying each commit on top of the upstream branch
/// 4. Updating the current branch reference to point to the final commit
///
/// The process maintains commit order but changes their parent relationships,
/// effectively "moving" the branch to start from the upstream commit.
pub async fn execute(args: RebaseArgs) {
    if !util::check_repo_exist() {
        return;
    }

    let db = get_db_conn_instance().await;

    // Get the current branch that will be moved to the new base
    let current_branch_name = match Head::current().await {
        Head::Branch(name) if !name.is_empty() => name,
        _ => {
            eprintln!("fatal: not on a branch or in detached HEAD state, cannot rebase");
            return;
        }
    };

    // Get the current HEAD commit that represents the tip of the branch to rebase
    let head_to_rebase_id = match Head::current_commit().await {
        Some(id) => id,
        None => {
            eprintln!("fatal: current branch '{current_branch_name}' has no commits");
            return;
        }
    };

    // Resolve the upstream reference to a concrete commit ID
    let upstream_id = match resolve_branch_or_commit(&args.upstream).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("fatal: {e}");
            return;
        }
    };

    // Find the merge base (common ancestor) between current branch and upstream
    // This determines which commits need to be replayed
    let base_id = match find_merge_base(&head_to_rebase_id, &upstream_id).await {
        Ok(Some(id)) => id,
        _ => {
            eprintln!("fatal: no common ancestor found");
            return;
        }
    };

    // Check if rebase is actually needed
    if base_id == head_to_rebase_id {
        println!(
            "Branch '{}' is already based on '{}'. No rebase needed.",
            current_branch_name, args.upstream
        );
        return;
    }
    if base_id == upstream_id {
        println!("Current branch is ahead of upstream. No rebase needed.");
        return;
    }

    // Collect all commits that need to be replayed from base to current HEAD
    let commits_to_replay = match collect_commits_to_replay(&base_id, &head_to_rebase_id).await {
        Ok(commits) if !commits.is_empty() => commits,
        _ => {
            println!("No commits to rebase on branch '{current_branch_name}'.",);
            return;
        }
    };
    println!("Found common ancestor: {}", &base_id.to_string()[..7]);
    println!(
        "Rebasing {} commits from '{}' onto '{}'...",
        commits_to_replay.len(),
        current_branch_name,
        args.upstream
    );

    let start_action = ReflogAction::Rebase {
        state: "start".to_string(),
        details: format!("checkout {}", args.upstream),
    };
    let start_context = ReflogContext {
        old_oid: head_to_rebase_id.to_string(),
        new_oid: upstream_id.to_string(),
        action: start_action,
    };
    let transaction_result = db.transaction(|txn| {
        Box::pin(async move {
            reflog::Reflog::insert_single_entry(txn, &start_context, "HEAD").await?;
            Head::update_with_conn(txn, Head::Detached(upstream_id), None).await;
            Ok::<_, ReflogError>(())
        })
    }).await;

    if let Err(e) = transaction_result {
        eprintln!("fatal: failed to start rebase: {}", e);
        return;
    }

    // This mimics Git's behavior.
    Head::update_with_conn(db, Head::Detached(upstream_id), None).await;
    // Replay each commit on top of the upstream branch
    // Each commit is applied as a three-way merge and creates a new commit
    println!(
        "Rebasing {} commits from `{}` onto `{}`...",
        commits_to_replay.len(), current_branch_name, args.upstream
    );
    let mut new_base_id = upstream_id;
    for commit_id in commits_to_replay {
        match replay_commit(&commit_id, &new_base_id).await {
            Ok(replayed_commit_id) => {
                new_base_id = replayed_commit_id;
                // Temporarily move HEAD along with each replayed commit.
                Head::update_with_conn(db, Head::Detached(new_base_id), None).await;
                let original_commit: Commit = load_object(&commit_id).unwrap();
                println!(
                    "Applied: {} {}",
                    &new_base_id.to_string()[..7],
                    original_commit.message.lines().next().unwrap_or("")
                );
            }
            Err(e) => {
                // IMPORTANT: If rebase failed, we should reset HEAD back to the original branch.
                Head::update_with_conn(db, Head::Branch(current_branch_name), None).await;
                eprintln!(
                    "error: could not apply {}: {}",
                    &commit_id.to_string()[..7],
                    e
                );
                eprintln!("Rebase failed. HEAD reset to original state.");
                return;
            }
        }
    }

    let final_commit_id = new_base_id;
    let finish_action = ReflogAction::Rebase {
        state: "finish".to_string(),
        details: format!("returning to refs/heads/{current_branch_name}"),
    };
    let finish_context = ReflogContext {
        old_oid: head_to_rebase_id.to_string(),
        new_oid: final_commit_id.to_string(),
        action: finish_action,
    };

    let branch_name_cloned = current_branch_name.clone();
    if let Err(e) = with_reflog(
        finish_context,
        move |txn: &sea_orm::DatabaseTransaction| {
            Box::pin(async move {
                // This is the crucial step: move the original branch from its old position
                // to the final replayed commit.
                Branch::update_branch_with_conn(txn, &branch_name_cloned, &final_commit_id.to_string(), None).await;

                // Also, re-attach HEAD to the newly moved branch.
                Head::update_with_conn(txn, Head::Branch(branch_name_cloned.clone()), None).await;
                Ok(())
            })
        },
        true,
    ).await
    {
        eprintln!("fatal: failed to finalize rebase: {e}");
        // Attempt to restore HEAD to a safe state
        Head::update_with_conn(db, Head::Detached(upstream_id), None).await;
    }

    // Reset the working directory and index to match the final state
    // This ensures that the workspace reflects the rebased commits
    let final_commit: Commit = load_object(&new_base_id).unwrap();
    let final_tree: Tree = load_object(&final_commit.tree_id).unwrap();

    let index_file = path::index();
    let mut index = mercury::internal::index::Index::new();
    rebuild_index_from_tree(&final_tree, &mut index, "").unwrap();
    index.save(&index_file).unwrap();
    reset_workdir_to_index(&index).unwrap();

    println!(
        "Successfully rebased branch '{}' onto '{}'.",
        current_branch_name, args.upstream
    );
}

/// Resolve a branch name or commit reference to a SHA1 hash
///
/// This function first tries to find a branch with the given name,
/// then falls back to resolving it as a commit reference (hash, HEAD, etc.).
/// This allows the rebase command to work with both branch names and commit hashes.
async fn resolve_branch_or_commit(reference: &str) -> Result<SHA1, String> {
    // First try to resolve as a branch name
    if let Some(branch) = Branch::find_branch(reference, None).await {
        return Ok(branch.commit);
    }
    // Fall back to commit hash resolution
    match util::get_commit_base(reference).await {
        Ok(id) => Ok(id),
        Err(_) => Err(format!("invalid reference: {reference}")),
    }
}

/// Replay a single commit on top of a new parent commit
///
/// This function performs a three-way merge to apply the changes from one commit
/// onto a different base commit. The three points of the merge are:
/// - Base: The original parent of the commit being replayed
/// - Theirs: The commit being replayed (contains the changes to apply)
/// - Ours: The new parent commit (where we want to apply the changes)
///
/// The result is a new commit with the same changes but a different parent.
async fn replay_commit(commit_to_replay_id: &SHA1, new_parent_id: &SHA1) -> Result<SHA1, String> {
    let commit_to_replay: Commit = load_object(commit_to_replay_id).map_err(|e| e.to_string())?;
    let original_parent_id = commit_to_replay
        .parent_commit_ids
        .first()
        .ok_or_else(|| "commit to replay has no parents".to_string())?;

    // Load the three trees needed for the three-way merge
    // Base tree: state before the original commit
    let base_tree: Tree = load_object(
        &load_object::<Commit>(original_parent_id)
            .map_err(|e| e.to_string())?
            .tree_id,
    )
    .map_err(|e| e.to_string())?;

    // Their tree: state after the original commit (the changes we want to apply)
    let their_tree: Tree = load_object(&commit_to_replay.tree_id).map_err(|e| e.to_string())?;

    // Our tree: current state of the new parent (where we apply changes)
    let our_tree: Tree = load_object(
        &load_object::<Commit>(new_parent_id)
            .map_err(|e| e.to_string())?
            .tree_id,
    )
    .map_err(|e| e.to_string())?;

    // Calculate what changed between base and their trees
    let diff = diff_trees(&their_tree, &base_tree);
    let mut merged_items: HashMap<PathBuf, SHA1> = our_tree.get_plain_items().into_iter().collect();

    // Apply the changes to our tree
    for (path, their_hash, _base_hash) in diff {
        if let Some(hash) = their_hash {
            // File was added or modified: use the new content
            merged_items.insert(path, hash);
        } else {
            // File was deleted: remove it from our tree
            merged_items.remove(&path);
        }
    }
    // TODO: Implement proper three-way merge with conflict detection
    // Currently using simplified logic that always takes "theirs" changes
    // without checking for conflicts with "ours" changes

    // Create a new tree with the merged content
    let new_tree_id = create_tree_from_items_map(&merged_items)?;

    // Create new commit with the same message but different parent and tree
    let new_commit =
        Commit::from_tree_id(new_tree_id, vec![*new_parent_id], &commit_to_replay.message);
    save_object(&new_commit, &new_commit.id).map_err(|e| e.to_string())?;
    Ok(new_commit.id)
}

/// Find the merge base (common ancestor) of two commits
///
/// This function implements a simple merge base algorithm:
/// 1. Traverse all ancestors of the first commit and store them in a set
/// 2. Traverse ancestors of the second commit until we find one in the set
/// 3. Return the first common ancestor found
///
/// Note: This returns the first common ancestor found, not necessarily the
/// best common ancestor. A more sophisticated algorithm would find the
/// lowest common ancestor (LCA).
///
/// TODO: Implement proper LCA algorithm for better merge base detection
/// TODO: Optimize performance for large repositories with many commits
async fn find_merge_base(commit1_id: &SHA1, commit2_id: &SHA1) -> Result<Option<SHA1>, String> {
    let mut visited1 = HashSet::new();
    let mut visited2 = HashSet::new();
    let mut queue1 = vec![*commit1_id];
    let mut queue2 = vec![*commit2_id];
    while !queue1.is_empty() || !queue2.is_empty() {
        // Process one level of ancestors for commit1
        if let Some(current_id) = queue1.pop() {
            if visited2.contains(&current_id) {
                return Ok(Some(current_id)); // Found common ancestor
            }
            if visited1.insert(current_id) {
                let commit: Commit = load_object(&current_id).map_err(|e| e.to_string())?;
                for parent_id in &commit.parent_commit_ids {
                    queue1.push(*parent_id);
                }
            }
        }
        // Process one level of ancestors for commit2
        if let Some(current_id) = queue2.pop() {
            if visited1.contains(&current_id) {
                return Ok(Some(current_id)); // Found common ancestor
            }
            if visited2.insert(current_id) {
                let commit: Commit = load_object(&current_id).map_err(|e| e.to_string())?;
                for parent_id in &commit.parent_commit_ids {
                    queue2.push(*parent_id);
                }
            }
        }
    }
    Ok(None)
}

/// Collect all commits from base (exclusive) to head (inclusive) that need to be replayed
///
/// This function walks backwards from the head commit to the base commit,
/// collecting all commits in between. These are the commits that will be
/// replayed onto the new upstream base.
///
/// The commits are returned in chronological order (oldest first) so they
/// can be replayed in the correct sequence.
async fn collect_commits_to_replay(base_id: &SHA1, head_id: &SHA1) -> Result<Vec<SHA1>, String> {
    let mut commits = Vec::new();
    let mut current_id = *head_id;

    // Walk backwards from head to base, collecting commit IDs
    while current_id != *base_id {
        commits.push(current_id);
        let commit: Commit = load_object(&current_id).map_err(|e| e.to_string())?;
        if commit.parent_commit_ids.is_empty() {
            break; // Reached root commit
        }
        current_id = commit.parent_commit_ids[0]; // Follow first parent
                                                  // TODO: Handle merge commits properly - currently only follows first parent
                                                  // This may miss commits in complex branch histories
    }

    // Reverse to get chronological order (oldest first)
    commits.reverse();
    Ok(commits)
}

/// Compute the differences between two tree objects
///
/// This function compares two trees and returns a list of all files that
/// differ between them. Each difference is represented as a tuple containing:
/// - PathBuf: The file path that differs
/// - Option<SHA1>: The file hash in the "theirs" tree (None if deleted)
/// - Option<SHA1>: The file hash in the "base" tree (None if newly added)
///
/// This is used to determine what changes need to be applied during replay.
fn diff_trees(theirs: &Tree, base: &Tree) -> Vec<(PathBuf, Option<SHA1>, Option<SHA1>)> {
    let their_items: HashMap<_, _> = theirs.get_plain_items().into_iter().collect();
    let base_items: HashMap<_, _> = base.get_plain_items().into_iter().collect();
    let all_paths: HashSet<_> = their_items.keys().chain(base_items.keys()).collect();
    let mut diffs = Vec::new();

    for path in all_paths {
        let their_hash = their_items.get(path).cloned();
        let base_hash = base_items.get(path).cloned();
        if their_hash != base_hash {
            diffs.push((path.clone(), their_hash, base_hash));
        }
    }
    diffs
}

/// Create a tree object from a flat map of file paths to content hashes
///
/// This function takes a HashMap of file paths and their content hashes,
/// and builds a proper Git tree structure. It handles:
/// - Grouping files by their parent directories
/// - Creating tree objects for each directory
/// - Recursively building the tree structure from root to leaves
///
/// Returns the SHA1 hash of the root tree object.
fn create_tree_from_items_map(items: &HashMap<PathBuf, SHA1>) -> Result<SHA1, String> {
    // Group files by their parent directories
    let mut entries_map: HashMap<PathBuf, Vec<mercury::internal::object::tree::TreeItem>> =
        HashMap::new();
    for (path, hash) in items {
        let item = mercury::internal::object::tree::TreeItem {
            mode: mercury::internal::object::tree::TreeItemMode::Blob,
            name: path.file_name().unwrap().to_str().unwrap().to_string(),
            id: *hash,
        };
        // TODO: Handle file modes properly - currently assumes all files are blobs
        let parent_dir = path.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
        entries_map.entry(parent_dir).or_default().push(item);
    }
    build_tree_recursively(Path::new(""), &mut entries_map)
}

/// Recursively build tree objects from a directory structure
///
/// This helper function processes a directory and all its subdirectories:
/// 1. Creates tree items for all files in the current directory
/// 2. Recursively processes subdirectories to create subtree objects  
/// 3. Combines files and subdirectories into a single tree object
/// 4. Saves the tree object and returns its hash
///
/// The algorithm works bottom-up, creating leaf trees first and then
/// combining them into parent trees.
fn build_tree_recursively(
    current_path: &Path,
    entries_map: &mut HashMap<PathBuf, Vec<mercury::internal::object::tree::TreeItem>>,
) -> Result<SHA1, String> {
    // Get all files/items in the current directory
    let mut current_items = entries_map.remove(current_path).unwrap_or_default();

    // Find all subdirectories that are children of current directory
    let subdirs: Vec<_> = entries_map
        .keys()
        .filter(|p| p.parent() == Some(current_path))
        .cloned()
        .collect();

    // Recursively process each subdirectory
    for subdir_path in subdirs {
        let subdir_name = subdir_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let subtree_hash = build_tree_recursively(&subdir_path, entries_map)?;

        // Add the subdirectory as a tree item
        current_items.push(mercury::internal::object::tree::TreeItem {
            mode: mercury::internal::object::tree::TreeItemMode::Tree,
            name: subdir_name,
            id: subtree_hash,
        });
    }

    // Create and save the tree object for this directory
    let tree = Tree::from_tree_items(current_items).map_err(|e| e.to_string())?;
    save_object(&tree, &tree.id).map_err(|e| e.to_string())?;
    Ok(tree.id)
}

/// Reset the working directory to match the given index state
///
/// This function synchronizes the working directory with the index by:
/// 1. Removing any files that exist in the working directory but not in the index
/// 2. Writing out all files that are tracked in the index to the working directory
/// 3. Creating necessary parent directories as needed
///
/// This ensures the working directory reflects the final rebased state.
fn reset_workdir_to_index(index: &mercury::internal::index::Index) -> Result<(), String> {
    let workdir = util::working_dir();
    let tracked_paths = index.tracked_files();
    let index_files_set: HashSet<_> = tracked_paths.iter().collect();

    // Remove files that are no longer tracked
    let all_files_in_workdir = util::list_workdir_files().unwrap_or_default();
    for path_from_root in all_files_in_workdir {
        if !index_files_set.contains(&path_from_root) {
            let full_path = workdir.join(path_from_root);
            if full_path.exists() {
                fs::remove_file(&full_path).map_err(|e| e.to_string())?;
                // TODO: Implement atomic file operations with rollback capability
                // TODO: Handle directory cleanup when all files are removed
            }
        }
    }

    // Write out all tracked files
    for path_buf in &tracked_paths {
        let path_str = path_buf.to_string_lossy();
        if let Some(entry) = index.get(&path_str, 0) {
            let blob = mercury::internal::object::blob::Blob::load(&entry.hash);
            let target_path = workdir.join(&*path_str);

            // Create parent directories if needed
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&target_path, &blob.data).map_err(|e| e.to_string())?;
            // TODO: Preserve file permissions and timestamps
            // TODO: Handle large files efficiently (streaming)
        }
    }
    Ok(())
}

/// Rebuild an index from a tree object by recursively adding all files
///
/// This function traverses a tree object and adds all files to the given index.
/// It handles both files (blobs) and subdirectories (trees) by:
/// 1. For files: Loading the blob and creating an index entry
/// 2. For subdirectories: Recursively processing the subtree
///
/// The prefix parameter tracks the current directory path during recursion.
fn rebuild_index_from_tree(
    tree: &Tree,
    index: &mut mercury::internal::index::Index,
    prefix: &str,
) -> Result<(), String> {
    for item in &tree.tree_items {
        let full_path = if prefix.is_empty() {
            item.name.clone()
        } else {
            format!("{}/{}", prefix, item.name)
        };

        if let mercury::internal::object::tree::TreeItemMode::Tree = item.mode {
            // Recursively process subdirectory
            let subtree: Tree = load_object(&item.id).map_err(|e| e.to_string())?;
            rebuild_index_from_tree(&subtree, index, &full_path)?;
        } else {
            // Add file to index
            let blob = mercury::internal::object::blob::Blob::load(&item.id);
            let entry = mercury::internal::index::IndexEntry::new_from_blob(
                full_path,
                item.id,
                blob.data.len() as u32,
            );
            // TODO: Handle different file modes (executable, symlinks, etc.)
            // TODO: Add proper error handling for corrupted blob objects
            index.add(entry);
        }
    }
    Ok(())
}
