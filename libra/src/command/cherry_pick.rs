use crate::command::{load_object, save_object};
use crate::internal::branch::Branch;
use crate::internal::head::Head;
use crate::utils::object_ext::BlobExt;
use crate::utils::object_ext::TreeExt;
use crate::utils::{path, util};
use clap::Parser;
use common::utils::format_commit_msg;
use mercury::hash::SHA1;
use mercury::internal::index::{Index, IndexEntry};
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Arguments for the cherry-pick command
#[derive(Parser, Debug)]
pub struct CherryPickArgs {
    /// Commits to cherry-pick
    #[clap(required = true)]
    pub commits: Vec<String>,

    /// Don't automatically commit the cherry-pick
    #[clap(short = 'n', long)]
    pub no_commit: bool,
}

/// Execute the cherry-pick command
///
/// Cherry-pick applies the changes introduced by some existing commits to the current branch.
/// This is useful for selectively applying commits from one branch to another without merging.
///
/// The process involves:
/// 1. Resolving commit references to SHA1 hashes
/// 2. For each commit, performing a three-way merge to apply changes
/// 3. Creating new commits with the same changes and the current HEAD as the parent
///    TODO: Now, it will apply the picked commits exactly as they are,
///    without resolving any conflicts, and it will not take the state of the staging area into account.
pub async fn execute(args: CherryPickArgs) {
    if !util::check_repo_exist() {
        return;
    }

    if let Head::Detached(_) = Head::current().await {
        eprintln!("fatal: cannot cherry-pick on detached HEAD");
        return;
    }

    // To simplify the implementation, we currently disallow cherry-picking multiple commits with --no-commit.
    // Unlike Git, which stages changes from all selected commits, this tool restricts the operation to a single commit.
    // This limitation may be lifted in the future if multi-commit support is implemented.
    if args.no_commit && args.commits.len() > 1 {
        eprintln!("fatal: cannot cherry-pick multiple commits with --no-commit");
        eprintln!("(use 'libra commit' to save the changes from the first cherry-pick)");
        return;
    }

    let mut commit_ids = Vec::new();
    for commit_ref in &args.commits {
        match resolve_commit(commit_ref).await {
            Ok(id) => commit_ids.push(id),
            Err(e) => {
                eprintln!("fatal: {e}");
                return;
            }
        }
    }

    for (i, commit_id) in commit_ids.iter().enumerate() {
        println!("Cherry-picking commit {}...", &commit_id.to_string()[..7]);
        match cherry_pick_single_commit(commit_id, &args).await {
            Ok(new_commit_id) => {
                if let Some(id) = new_commit_id {
                    println!(
                        "Finished cherry-pick for {} in new commit {}",
                        &commit_id.to_string()[..7],
                        &id.to_string()[..7]
                    );
                } else {
                    println!("Changes staged for cherry-pick. Use 'libra commit' to finalize.");
                }
            }
            Err(e) => {
                eprintln!("error: failed to cherry-pick {}: {}", &args.commits[i], e);
                // This simplified implementation does not handle conflicts or offer options to abort or skip.
                return;
            }
        }
    }
}

/// Cherry-pick a single commit onto the current branch
///
/// This function performs the core cherry-pick logic:
/// 1. Loads the commit to be cherry-picked and its parent
/// 2. Performs a three-way merge between:
///    - Base: The parent of the commit being cherry-picked
///    - Theirs: The commit being cherry-picked
///    - Ours: The current HEAD state
/// 3. Applies the resulting changes to the index and working directory
/// 4. Optionally creates a new commit with the applied changes
///
/// Returns the SHA1 of the new commit if created, or None if --no-commit was used
async fn cherry_pick_single_commit(
    commit_id: &SHA1,
    args: &CherryPickArgs,
) -> Result<Option<SHA1>, String> {
    let commit_to_pick: Commit =
        load_object(commit_id).map_err(|e| format!("failed to load commit: {e}"))?;

    if commit_to_pick.parent_commit_ids.len() > 1 {
        return Err("cherry-picking merge commits is not supported".to_string());
    }

    // Three-way merge key points:
    // Base: Parent commit of the commit to cherry-pick
    // Theirs: The commit to cherry-pick
    // Ours: Current HEAD
    let parent_tree = if commit_to_pick.parent_commit_ids.is_empty() {
        Tree::from_tree_items(vec![]).unwrap() // Root commit, base is an empty tree
    } else {
        let parent_commit: Commit = load_object(&commit_to_pick.parent_commit_ids[0])
            .map_err(|e| format!("failed to load parent commit: {e}"))?;
        load_object(&parent_commit.tree_id)
            .map_err(|e| format!("failed to load parent tree: {e}"))?
    };

    let their_tree: Tree = load_object(&commit_to_pick.tree_id)
        .map_err(|e| format!("failed to load commit tree: {e}"))?;

    let index_file = path::index();
    let mut index =
        Index::load(&index_file).map_err(|e| format!("failed to load current index: {e}"))?;

    // Apply patch to current index
    // 1. Get diff (theirs vs base)
    let diff = diff_trees(&their_tree, &parent_tree);

    // 2. Apply diff to current index (simplified merge)
    for (path, their_hash, base_hash) in diff {
        match (their_hash, base_hash) {
            (Some(th), Some(_bh)) => {
                // Modified file
                update_index_entry(&mut index, &path, th);
            }
            (Some(th), None) => {
                // New file
                update_index_entry(&mut index, &path, th);
            }
            (None, Some(_bh)) => {
                // Deleted file
                index.remove(path.to_str().unwrap(), 0);
            }
            (None, None) => unreachable!(),
        }
    }

    // 3. Save updated index and sync working directory
    index
        .save(&index_file)
        .map_err(|e| format!("failed to save index: {e}"))?;
    reset_workdir_to_index(&index)?;

    if args.no_commit {
        Ok(None)
    } else {
        let current_head = Head::current_commit()
            .await
            .ok_or("failed to resolve current HEAD")?;
        let cherry_pick_commit_id =
            create_cherry_pick_commit(&commit_to_pick, &current_head).await?;
        Ok(Some(cherry_pick_commit_id))
    }
}

/// Create a new commit representing the cherry-picked changes
///
/// This function:
/// 1. Creates a tree object from the current index state
/// 2. Generates a commit message indicating the original commit
/// 3. Creates a new commit object with the current HEAD as parent
/// 4. Updates the HEAD reference to point to the new commit
async fn create_cherry_pick_commit(
    original_commit: &Commit,
    parent_id: &SHA1,
) -> Result<SHA1, String> {
    let index = Index::load(path::index()).map_err(|e| format!("failed to load index: {e}"))?;

    // Create tree from current index state
    let tree_id = create_tree_from_index(&index)?;

    let cherry_pick_message = format!(
        "{}\n\n(cherry picked from commit {})",
        original_commit.message.trim(),
        original_commit.id
    );

    let commit = Commit::from_tree_id(
        tree_id,
        vec![*parent_id],
        &format_commit_msg(&cherry_pick_message, None),
    );

    save_object(&commit, &commit.id).map_err(|e| format!("failed to save commit: {e}"))?;
    update_head(&commit.id.to_string()).await;
    Ok(commit.id)
}

/// Calculate differences between two trees to generate a patch
///
/// This function compares two tree objects and returns a list of differences.
/// Each difference is represented as a tuple containing:
/// - PathBuf: The file path
/// - Option<SHA1>: The hash in the "theirs" tree (None if deleted)
/// - Option<SHA1>: The hash in the "base" tree (None if newly added)
///
/// This is used to determine what changes need to be applied when cherry-picking.
fn diff_trees(theirs: &Tree, base: &Tree) -> Vec<(PathBuf, Option<SHA1>, Option<SHA1>)> {
    let mut diffs = Vec::new();
    let their_items: HashMap<_, _> = theirs.get_plain_items().into_iter().collect();
    let base_items: HashMap<_, _> = base.get_plain_items().into_iter().collect();

    let all_paths: HashSet<_> = their_items.keys().chain(base_items.keys()).collect();

    for path in all_paths {
        let their_hash = their_items.get(path).cloned();
        let base_hash = base_items.get(path).cloned();
        if their_hash != base_hash {
            diffs.push((path.clone(), their_hash, base_hash));
        }
    }
    diffs
}

/// Add or update a file entry in the index
///
/// This function:
/// 1. Loads the blob object from the given hash
/// 2. Creates a new IndexEntry with the file path, hash, and size
/// 3. Adds the entry to the index (overwriting any existing entry with the same path)
///
/// This is used when applying cherry-pick changes to update the index state.
fn update_index_entry(index: &mut Index, path: &Path, hash: SHA1) {
    let blob = mercury::internal::object::blob::Blob::load(&hash);
    let entry = IndexEntry::new_from_blob(
        path.to_str().unwrap().to_string(),
        hash,
        blob.data.len() as u32,
    );
    index.add(entry); // add() will overwrite entries with the same name
}

/// Create a tree object from the current index state
///
/// This function builds a complete tree structure representing the current index:
/// 1. Groups index entries by their parent directories
/// 2. Recursively builds tree objects starting from the root
/// 3. Each directory becomes a tree object containing its files and subdirectories
///
/// This is a reusable utility function that can be used in commit.rs and revert.rs
/// as well, since creating trees from index state is a common operation.
///
/// Returns the SHA1 hash of the root tree object.
fn create_tree_from_index(index: &Index) -> Result<SHA1, String> {
    // Path -> TreeItem mapping
    let mut entries_map: HashMap<PathBuf, Vec<TreeItem>> = HashMap::new();
    for path_buf in index.tracked_files() {
        let path_str = path_buf.to_str().unwrap();
        if let Some(entry) = index.get(path_str, 0) {
            let item = TreeItem {
                mode: match entry.mode {
                    0o100644 => TreeItemMode::Blob,           // Regular file
                    0o100755 => TreeItemMode::BlobExecutable, // Executable file
                    0o120000 => TreeItemMode::Link,           // Symbolic link
                    0o040000 => TreeItemMode::Tree,           // Directory
                    _ => return Err(format!("Unsupported file mode: {:#o}", entry.mode)),
                },
                name: path_buf.file_name().unwrap().to_str().unwrap().to_string(),
                id: entry.hash,
            };
            let parent_dir = path_buf
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_path_buf();
            entries_map.entry(parent_dir).or_default().push(item);
        }
    }

    // Build recursively
    build_tree_recursively(Path::new(""), &mut entries_map)
}

/// Recursively build tree objects from a directory structure map
///
/// This helper function:
/// 1. Takes the current directory path and the remaining entries map
/// 2. Creates tree items for all files in the current directory
/// 3. Recursively processes subdirectories to create subtree objects
/// 4. Combines everything into a single tree object for the current directory
///
/// The recursion builds the tree structure bottom-up, creating leaf trees first
/// and then combining them into parent trees.
fn build_tree_recursively(
    current_path: &Path,
    entries_map: &mut HashMap<PathBuf, Vec<TreeItem>>,
) -> Result<SHA1, String> {
    let mut current_items = entries_map.remove(current_path).unwrap_or_default();

    // Find all subdirectories and build them recursively
    let subdirs: Vec<_> = entries_map
        .keys()
        .filter(|p| p.parent() == Some(current_path))
        .cloned()
        .collect();

    for subdir_path in subdirs {
        let subdir_name = subdir_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let subtree_hash = build_tree_recursively(&subdir_path, entries_map)?;
        current_items.push(TreeItem {
            mode: TreeItemMode::Tree,
            name: subdir_name,
            id: subtree_hash,
        });
    }

    let tree =
        Tree::from_tree_items(current_items).map_err(|e| format!("failed to create tree: {e}"))?;
    save_object(&tree, &tree.id).map_err(|e| e.to_string())?;
    Ok(tree.id)
}

/// Reset the working directory to match the current index state
///
/// This function synchronizes the working directory with the index by:
/// 1. Removing any files that exist in the working directory but not in the index
/// 2. Writing out all files that are tracked in the index to the working directory
/// 3. Creating necessary parent directories as needed
///
/// This ensures that after cherry-picking, the working directory reflects the
/// merged state that was applied to the index.
fn reset_workdir_to_index(index: &Index) -> Result<(), String> {
    let workdir = util::working_dir();
    let tracked_paths = index.tracked_files();
    let index_files_set: HashSet<_> = tracked_paths.iter().collect();
    let all_files_in_workdir = util::list_workdir_files().unwrap_or_default();
    for path_from_root in all_files_in_workdir {
        if !index_files_set.contains(&path_from_root) {
            let full_path = workdir.join(path_from_root);
            if full_path.exists() {
                fs::remove_file(&full_path).map_err(|e| e.to_string())?;
            }
        }
    }
    for path_buf in &tracked_paths {
        let path_str = path_buf.to_str().unwrap();
        if let Some(entry) = index.get(path_str, 0) {
            let blob = mercury::internal::object::blob::Blob::load(&entry.hash);
            let target_path = workdir.join(path_str);
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&target_path, &blob.data).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Resolve a commit reference (like "HEAD", branch name, or SHA1) to a SHA1 hash
///
/// This function uses the utility function to convert various commit references
/// into their corresponding SHA1 hashes. It supports:
/// - Full SHA1 hashes
/// - Abbreviated SHA1 hashes  
/// - Branch names
/// - Special references like "HEAD"
async fn resolve_commit(reference: &str) -> Result<SHA1, String> {
    util::get_commit_base(reference).await
}

/// Update the HEAD reference to point to a new commit
///
/// This function updates the current branch to point to the specified commit.
/// It only works when HEAD is pointing to a branch (not in detached HEAD state).
/// The branch reference is updated to the new commit ID.
async fn update_head(commit_id: &str) {
    if let Head::Branch(name) = Head::current().await {
        Branch::update_branch(&name, commit_id, None).await;
    }
}
