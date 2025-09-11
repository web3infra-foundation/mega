use crate::cli::Stash;
use crate::command::reset::{
    rebuild_index_from_tree, remove_empty_directories, reset_index_to_commit,
    restore_working_directory_from_tree,
};
use crate::internal::head::Head;
use crate::utils::object_ext::TreeExt;
use crate::utils::{object, tree, util};
use colored::Colorize;
use mercury::errors::GitError;
use mercury::hash::SHA1;
use mercury::internal::index::{Index, Time};
use mercury::internal::object::commit::Commit;
use mercury::internal::object::signature::Signature;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use mercury::internal::object::ObjectTrait;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub async fn execute(stash_cmd: Stash) {
    let result = match stash_cmd {
        Stash::Push { message } => push(message).await,
        Stash::Pop { stash } => pop(stash).await,
        Stash::List => list().await,
        Stash::Apply { stash } => apply(stash).await,
        Stash::Drop { stash } => drop_stash(stash).await,
        Stash::Branch { stash, branch } => branch(stash, branch).await,
    };

    if let Err(e) = result {
        eprintln!("{}", format!("fatal: {}", e).red());
    }
}

async fn push(message: Option<String>) -> Result<(), String> {
    if !has_changes().await {
        eprintln!("No local changes to save");
        return Ok(());
    }

    let git_dir = util::try_get_storage_path(None).map_err(|e| e.to_string())?;
    let index_path = git_dir.join("index");
    let index = Index::load(&index_path).unwrap_or_else(|_| Index::new());

    // Handle the special case of stashing in an empty repository (no HEAD commit)
    if Head::current_commit().await.is_none() {
        return Err("You do not have the initial commit yet".to_string());
    }

    // --- Standard stash process for a repository with commits ---

    // 1. Get parent commit (HEAD) and index tree
    let head_commit_hash = Head::current_commit()
        .await
        .ok_or_else(|| "Could not get HEAD commit hash".to_string())?;
    let head_commit_hash_str = head_commit_hash.to_string();

    let index_tree = tree::create_tree_from_index(&index).map_err(|e| e.to_string())?;
    let index_tree_hash = index_tree.id;

    // 2. Create the index commit object in memory
    let (author, committer) = util::create_signatures().await;
    let (current_branch_name, head_commit_summary) = match Head::current().await {
        Head::Branch(name) => {
            let head_commit_object_data =
                object::read_git_object(&git_dir, &head_commit_hash).map_err(|e| e.to_string())?;
            let head_commit = Commit::from_bytes(&head_commit_object_data, head_commit_hash)
                .map_err(|e| e.to_string())?;
            let summary = head_commit.message.lines().next().unwrap_or("").to_string();
            (name, summary)
        }
        Head::Detached(_) => {
            let head_commit_object_data =
                object::read_git_object(&git_dir, &head_commit_hash).map_err(|e| e.to_string())?;
            let head_commit = Commit::from_bytes(&head_commit_object_data, head_commit_hash)
                .map_err(|e| e.to_string())?;
            let summary = head_commit.message.lines().next().unwrap_or("").to_string();
            ("(no branch)".to_string(), summary)
        }
    };

    let wip_message = format!(
        "WIP on {}: {} {}",
        current_branch_name,
        &head_commit_hash_str[..7],
        head_commit_summary
    );
    let final_message = message.unwrap_or(wip_message);

    let index_commit = Commit::new(
        author.clone(),
        committer.clone(),
        index_tree_hash,
        vec![head_commit_hash],
        &final_message,
    );

    // 3. Write the index commit to the object database
    let data = index_commit.to_data().map_err(|e| e.to_string())?;
    let index_commit_hash =
        object::write_git_object(&git_dir, "commit", &data).map_err(|e| e.to_string())?;

    // 4. Create the worktree tree object
    let workdir = git_dir
        .parent()
        .ok_or_else(|| "Cannot find workdir".to_string())?;
    let worktree_tree = create_tree_from_workdir(workdir, &git_dir, &index)?;
    let worktree_tree_data = worktree_tree.to_data().map_err(|e| e.to_string())?;
    let worktree_tree_hash = object::write_git_object(&git_dir, "tree", &worktree_tree_data)
        .map_err(|e| e.to_string())?;

    // 5. Create the final stash commit
    let stash_commit = Commit::new(
        author,            // The original author signature
        committer.clone(), // CLONE the committer to retain ownership
        worktree_tree_hash,
        vec![head_commit_hash, index_commit_hash],
        &final_message,
    );
    let stash_commit_data = stash_commit.to_data().map_err(|e| e.to_string())?;
    let stash_commit_hash = object::write_git_object(&git_dir, "commit", &stash_commit_data)
        .map_err(|e| e.to_string())?;

    // 6. Update refs/stash ref
    update_stash_ref(&git_dir, &stash_commit_hash, &committer, &final_message)
        .map_err(|e| e.to_string())?;

    println!("Saved working directory and index state {}", final_message);

    // 7. Reset the working directory and index to the original HEAD
    perform_hard_reset(&head_commit_hash).await?;
    Ok(())
}

async fn pop(stash: Option<String>) -> Result<(), String> {
    // First, apply the stash.
    do_apply(stash.clone()).await?;

    // If apply was successful, drop the stash.
    // We use the original `stash` Option<String> for the drop command.
    drop_stash(stash).await
}

async fn list() -> Result<(), String> {
    if !has_stash() {
        // git stash list 在没有 stash 时不输出任何内容，所以这里直接返回是正确的行为。
        return Ok(());
    }

    let git_dir = util::try_get_storage_path(None).map_err(|e| e.to_string())?;
    let stash_log_path = git_dir.join("logs/refs/stash");
    if !stash_log_path.exists() {
        return Ok(());
    }
    let file = std::fs::File::open(stash_log_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);

    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    for (index, line_content) in lines.iter().enumerate() {
        // reflog format: <old_hash> <new_hash> Author <email> timestamp <tz>	message
        // We need the message part.
        let parts: Vec<&str> = line_content.splitn(2, '\t').collect();
        if parts.len() == 2 {
            let message = parts[1];
            println!("stash@{{{}}}: {}", index, message);
        }
    }
    Ok(())
}

async fn apply(stash: Option<String>) -> Result<(), String> {
    do_apply(stash).await?;
    Ok(())
}
async fn branch(stash: Option<String>, branch_name: String) -> Result<(), String> {
    // 🔍 Check 1: Verify stash exists
    if !has_stash() {
        return Err("fatal: No stash entries found".to_string());
    }

    // 🔍 Check 2: Parse and validate stash reference
    let (stash_index, stash_commit_hash) = resolve_stash_to_commit_hash(stash)?;

    // 🔍 Check 3 & 4: Validate branch name and check if it already exists
    if let Err(e) = validate_and_check_branch(&branch_name).await {
        return Err(e);
    }

    // 🎯 CORRECT LOGIC: Create branch directly from stash commit
    println!("Creating branch '{}' from stash@{{{}}}", branch_name, stash_index);
    
    // Create branch pointing to the stash commit itself, not its parent
    match crate::internal::branch::update_branch(
        &branch_name, 
        &stash_commit_hash.to_string(), 
        None
    ).await {
        Ok(_) => {},
        Err(e) => {
            eprintln!("error: Failed to create branch '{}': {}", branch_name, e);
            return Err(format!("fatal: Failed to create branch '{}': {}", branch_name, e));
        },
    }

    // 🎯 CORRECT LOGIC: Switch to the new branch
    println!("Switching to branch '{}'", branch_name);
    match command::switch::execute(command::switch::SwitchArgs {
        branch: branch_name.clone(),
        create: false,
        force: false,
        guess: false,
    }).await {
        Ok(_) => {},
        Err(e) => {
            eprintln!("error: Failed to switch to branch '{}': {}", branch_name, e);

            // Rollback: Delete the created branch to maintain consistency
            if let Err(rollback_err) = crate::internal::branch::delete(&branch_name).await {
                eprintln!("error: Failed to rollback branch deletion: {}", rollback_err);
            }

            return Err(format!("fatal: Failed to switch to branch '{}': {}", branch_name, e));
        },
    }

    // 🎯 CORRECT LOGIC: Drop the stash (changes are now in the branch)
    println!("Dropping stash@{{{}}}", stash_index);
    if let Err(e) = drop_stash(Some(format!("stash@{{{}}}", stash_index))).await {
        eprintln!("warning: Failed to drop stash: {}", e);
        return Err(format!("non-fatal: Failed to drop stash@{{{}}}. Please clean up manually.", stash_index));
    }

    // 🎉 Success message
    println!("Successfully created branch '{}' from stash@{{{}}}", branch_name, stash_index);
    println!("Branch '{}' is now checked out and stash has been dropped.", branch_name);
    
    Ok(())
}

// New helper function to combine branch validation and existence check

async fn validate_and_check_branch(branch_name: &str) -> Result<(), String> {
    
    if !crate::command::branch::is_valid_git_branch_name(branch_name) {
        return Err(format!("fatal: '{}' is not a valid branch name", branch_name));
    }
    
  
    if crate::internal::branch::exists(branch_name).await {
        return Err(format!("fatal: A branch named '{}' already exists", branch_name));
    }
    Ok(())
}

/// Helper function containing the core logic for applying a stash.
/// Returns true on success, false on failure.
async fn do_apply(stash: Option<String>) -> Result<(), String> {
    let (index, hash_str) = resolve_stash_to_commit_hash(stash)?;
    let stash_commit_hash = SHA1::from_str(&hash_str).map_err(|e| e.to_string())?;
    let git_dir = util::try_get_storage_path(None).map_err(|e| e.to_string())?;

    println!("Applying stash@{{{}}}...", index);

    // Load the stash commit
    let stash_commit_data =
        object::read_git_object(&git_dir, &stash_commit_hash).map_err(|e| e.to_string())?;
    let stash_commit =
        Commit::from_bytes(&stash_commit_data, stash_commit_hash).map_err(|e| e.to_string())?;

    // Handle stashes in an empty repository

    // --- Three-way Merge Logic for Stash Apply ---
    let base_commit_hash = *stash_commit
        .parent_commit_ids
        .first()
        .ok_or("Stash commit is malformed and has no base parent")?;
    let head_commit_hash = Head::current_commit()
        .await
        .ok_or_else(|| "Could not get HEAD commit hash".to_string())?;

    // Load the necessary commits and trees
    let base_commit_data =
        object::read_git_object(&git_dir, &base_commit_hash).map_err(|e| e.to_string())?;
    let base_commit =
        Commit::from_bytes(&base_commit_data, base_commit_hash).map_err(|e| e.to_string())?;
    let base_tree_data =
        object::read_git_object(&git_dir, &base_commit.tree_id).map_err(|e| e.to_string())?;
    let base_tree =
        Tree::from_bytes(&base_tree_data, base_commit.tree_id).map_err(|e| e.to_string())?;

    let head_commit_data =
        object::read_git_object(&git_dir, &head_commit_hash).map_err(|e| e.to_string())?;
    let head_commit =
        Commit::from_bytes(&head_commit_data, head_commit_hash).map_err(|e| e.to_string())?;
    let head_tree_data =
        object::read_git_object(&git_dir, &head_commit.tree_id).map_err(|e| e.to_string())?;
    let head_tree =
        Tree::from_bytes(&head_tree_data, head_commit.tree_id).map_err(|e| e.to_string())?;

    let stash_tree_data =
        object::read_git_object(&git_dir, &stash_commit.tree_id).map_err(|e| e.to_string())?;
    let stash_tree =
        Tree::from_bytes(&stash_tree_data, stash_commit.tree_id).map_err(|e| e.to_string())?;

    // Perform the tree merge
    let merged_tree = merge_trees(&base_tree, &head_tree, &stash_tree, &git_dir)?;

    // Update working directory and index based on the merged tree
    let workdir = git_dir.parent().unwrap();
    let index_path = git_dir.join("index");
    let mut index = Index::new();

    // Clean the working directory based on changes between HEAD and the merged tree
    let head_files = tree::get_tree_files_recursive(&head_tree, &git_dir, &PathBuf::new())?;
    let merged_files = tree::get_tree_files_recursive(&merged_tree, &git_dir, &PathBuf::new())?;

    for (path, _) in head_files.iter() {
        if !merged_files.contains_key(path) {
            let full_path = workdir.join(path);
            if full_path.exists() {
                fs::remove_file(full_path).map_err(|e| e.to_string())?;
            }
        }
    }

    restore_working_directory_from_tree(&merged_tree, workdir, "")?;
    rebuild_index_from_tree(&merged_tree, &mut index, "")?;

    index
        .save(&index_path)
        .map_err(|e| format!("Failed to save index: {}", e))?;

    let current_branch_name = match Head::current().await {
        Head::Branch(name) => name,
        Head::Detached(_) => "(no branch)".to_string(),
    };
    println!(
        "On branch {}\nChanges not staged for commit:\n  (use \"git add <file>...\" to update what will be committed)\n  (use \"git restore <file>...\" to discard changes in working directory)\n",
        current_branch_name
    );
    Ok(())
}

async fn drop_stash(stash: Option<String>) -> Result<(), String> {
    if !has_stash() {
        return Err("No stash found".to_string());
    }

    let git_dir = util::try_get_storage_path(None).map_err(|e| e.to_string())?;
    let stash_ref_path = git_dir.join("refs/stash");
    let stash_log_path = git_dir.join("logs/refs/stash");

    // Read all lines from the stash reflog
    let file = std::fs::File::open(&stash_log_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader
        .lines()
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;

    // Determine which stash to drop
    let index_to_drop = match stash {
        None => 0, // Default to the latest stash (stash@{0})
        Some(s) => {
            if s.starts_with("stash@{") && s.ends_with('}') {
                s[7..s.len() - 1]
                    .parse::<usize>()
                    .map_err(|_| format!("'{}' is not a valid stash reference", s))?
            } else {
                return Err(format!("'{}' is not a valid stash reference", s));
            }
        }
    };

    // Remove the corresponding line. stash@{0} is the first line in the file.
    if index_to_drop >= lines.len() {
        return Err(format!("stash@{{{}}}: Stash does not exist", index_to_drop));
    }
    let removed_line = lines.remove(index_to_drop);

    // Get commit hash for the confirmation message
    let stash_commit_hash = removed_line.split(' ').nth(1).unwrap_or("unknown");
    println!(
        "Dropped stash@{{{}}} ({})",
        index_to_drop, stash_commit_hash
    );

    // Write the remaining lines back, or delete the file if it's empty
    if lines.is_empty() {
        std::fs::remove_file(&stash_log_path).map_err(|e| e.to_string())?;
        if stash_ref_path.exists() {
            std::fs::remove_file(&stash_ref_path).map_err(|e| e.to_string())?;
        }
    } else {
        let new_content = lines.join("\n") + "\n";
        std::fs::write(&stash_log_path, new_content).map_err(|e| e.to_string())?;

        // If we dropped the top of the stack, update the main ref
        if index_to_drop == 0 {
            if let Some(new_top_line) = lines.first() {
                if let Some(new_hash) = new_top_line.split(' ').nth(1) {
                    std::fs::write(&stash_ref_path, format!("{new_hash}\n"))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }
    Ok(())
}

// Checks for staged changes by comparing the HEAD tree with the index tree.
async fn has_changes() -> bool {
    let Some(git_dir) = util::try_get_storage_path(None).ok() else {
        return false;
    };

    // Get the tree hash from the HEAD commit.
    let head_tree_hash = match Head::current_commit().await {
        Some(hash) => {
            let Ok(commit_data) = object::read_git_object(&git_dir, &hash) else {
                return false;
            };
            let Ok(commit) = Commit::from_bytes(&commit_data, hash) else {
                return false;
            };
            commit.tree_id
        }
        None => {
            // No HEAD commit yet (empty repository). Compare against the empty tree.
            SHA1::from_str("4b825dc642cb6eb9a060e54bf8d69288fbee4904").unwrap()
        }
    };

    // Get the tree hash from the current index.
    let index_path = git_dir.join("index");
    let Ok(index) = Index::load(&index_path) else {
        return false;
    };
    let Ok(index_tree) = tree::create_tree_from_index(&index) else {
        return false;
    };
    let index_tree_hash = index_tree.id;

    // If the hashes are different, there are staged changes.
    if head_tree_hash != index_tree_hash {
        return true;
    }

    let workdir = git_dir.parent().unwrap();
    for entry in index.tracked_entries(0) {
        let file_path = workdir.join(&entry.name);

        // Check if the file exists on disk. If not, it's a deletion.
        let Ok(metadata) = fs::metadata(&file_path) else {
            return true; // File deleted from workdir
        };

        // Quick check: if mtime and size are the same, assume the file is unchanged.
        let mtime =
            Time::from_system_time(metadata.modified().unwrap_or(std::time::SystemTime::now()));
        if metadata.len() == entry.size as u64 && mtime == entry.mtime {
            continue;
        }

        // Definitive check: compare content hash.
        if let Ok(content) = fs::read(&file_path) {
            let header = format!("blob {}\0", content.len());
            let mut full_content = header.into_bytes();
            full_content.extend_from_slice(&content);
            let current_hash = SHA1::new(&full_content);

            if current_hash != entry.hash {
                return true; // Content is different, definitely modified.
            }
        } else {
            return true; // Cannot read file, treat as a change
        }
    }

    false
}

fn has_stash() -> bool {
    util::try_get_storage_path(None)
        .ok()
        .map(|p| p.join("refs/stash").is_file())
        .unwrap_or(false)
}

/// Resolves a stash reference (e.g., "stash@{1}") to its index and commit hash.
/// If the reference is None, it resolves the latest stash (stash@{0}).
fn resolve_stash_to_commit_hash(stash_ref: Option<String>) -> Result<(usize, String), String> {
    if !has_stash() {
        return Err("No stash found".to_string());
    }

    let git_dir = util::try_get_storage_path(None).unwrap();
    let stash_log_path = git_dir.join("logs/refs/stash");
    if !stash_log_path.exists() {
        return Err("No stash found".to_string());
    }

    let file = std::fs::File::open(&stash_log_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    let index_to_resolve = match stash_ref {
        None => 0,
        Some(s) => {
            if s.starts_with("stash@{") && s.ends_with('}') {
                s[7..s.len() - 1]
                    .parse::<usize>()
                    .map_err(|_| format!("fatal: '{s}' is not a valid stash reference"))?
            } else {
                return Err(format!("fatal: '{s}' is not a valid stash reference"));
            }
        }
    };

    if index_to_resolve >= lines.len() {
        return Err(format!(
            "fatal: stash@{{{index_to_resolve}}}: Stash does not exist",
        ));
    }

    let line_content = &lines[index_to_resolve];

    // reflog format: <old_hash> <new_hash> ...
    // The stash commit is the new_hash.
    let commit_hash = line_content
        .split(' ')
        .nth(1)
        .ok_or_else(|| "fatal: Corrupted stash log".to_string())?;

    Ok((index_to_resolve, commit_hash.to_string()))
}

/// Updates the stash ref and its reflog.
fn update_stash_ref(
    git_dir: &Path,
    stash_hash: &SHA1,
    committer: &Signature,
    message: &str,
) -> Result<(), GitError> {
    let stash_ref_path = git_dir.join("refs/stash");
    let stash_log_path = git_dir.join("logs/refs/stash");

    // 1. Get old hash from refs/stash
    let old_hash = if stash_ref_path.exists() {
        let content = fs::read_to_string(&stash_ref_path)?;
        SHA1::from_str(content.trim())
            .map_err(|_| GitError::InvalidHashValue(content.trim().to_string()))?
    } else {
        SHA1::default() // Null hash
    };

    // 2. Write new hash to refs/stash
    if let Some(parent) = stash_ref_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&stash_ref_path, format!("{stash_hash}\n"))?;

    // 3. Prepend to logs/refs/stash
    if let Some(parent) = stash_log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let reflog_entry = format!(
        "{} {} {} <{}> {} {}\t{}",
        old_hash,
        stash_hash,
        committer.name,
        committer.email,
        committer.timestamp,
        committer.timezone,
        message
    );

    let mut lines = if stash_log_path.exists() {
        let content = fs::read_to_string(&stash_log_path)?;
        content.lines().map(String::from).collect()
    } else {
        Vec::new()
    };

    lines.insert(0, reflog_entry);
    let new_content = lines.join("\n") + "\n";
    fs::write(stash_log_path, new_content)?;

    Ok(())
}

async fn perform_hard_reset(target_commit_id: &SHA1) -> Result<(), String> {
    let git_dir = util::try_get_storage_path(None).map_err(|e| e.to_string())?;
    let workdir = git_dir
        .parent()
        .ok_or_else(|| "Cannot find workdir".to_string())?;
    let index_path = git_dir.join("index");

    // 1. Get the list of all files that are currently tracked (before reset)
    let index_before_reset = Index::load(&index_path).unwrap_or_else(|_| Index::new());
    let all_tracked_paths: Vec<PathBuf> = index_before_reset
        .tracked_entries(0)
        .into_iter()
        .map(|e| PathBuf::from(&e.name))
        .collect();

    // 2. Get the list of files in the target commit's tree
    let target_commit: Commit = crate::command::load_object(target_commit_id)
        .map_err(|e| format!("failed to load target commit: {e}"))?;
    let target_tree: Tree = crate::command::load_object(&target_commit.tree_id)
        .map_err(|e| format!("failed to load target tree: {e}"))?;
    let files_in_target_tree: HashSet<PathBuf> = target_tree
        .get_plain_items()
        .into_iter()
        .map(|(p, _)| p)
        .collect();

    // 3. Reset index to the target commit's tree
    reset_index_to_commit(target_commit_id)?;

    // 4. Clean the working directory by removing files that were tracked before but are not in the target commit
    for path in &all_tracked_paths {
        if !files_in_target_tree.contains(path) {
            let full_path = workdir.join(path);
            if full_path.exists() {
                fs::remove_file(full_path).map_err(|e| format!("Failed to remove file: {e}"))?;
            }
        }
    }

    // 5. Restore/update working directory files from the target commit's tree
    restore_working_directory_from_tree(&target_tree, workdir, "")?;

    // 6. Clean up empty directories that might be left behind
    remove_empty_directories(workdir)?;

    Ok(())
}

/// Creates a `Tree` object from the files in the working directory.
fn create_tree_from_workdir(workdir: &Path, git_dir: &Path, index: &Index) -> Result<Tree, String> {
    fn build_tree_recursive(
        dir: &Path,
        git_dir: &Path,
        index: &Index,
        workdir: &Path,
    ) -> Result<Tree, String> {
        let mut items = Vec::new();
        let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;

        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_str().unwrap().to_string();

            // Ignore the .libra directory
            if path.ends_with(".libra") {
                continue;
            }

            if path.is_dir() {
                let subtree = build_tree_recursive(&path, git_dir, index, workdir)?;
                let subtree_data = subtree.to_data().map_err(|e| e.to_string())?;
                let subtree_hash = object::write_git_object(git_dir, "tree", &subtree_data)
                    .map_err(|e| e.to_string())?;
                items.push(TreeItem::new(TreeItemMode::Tree, subtree_hash, file_name));
            } else if path.is_file() {
                let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
                let relative_path = path.strip_prefix(workdir).unwrap();
                let relative_path_str = relative_path.to_str().unwrap();

                if let Some(entry) = index.get(relative_path_str, 0) {
                    let mtime = Time::from_system_time(
                        metadata.modified().unwrap_or(std::time::SystemTime::now()),
                    );
                    let size = metadata.len() as u32;

                    if entry.mtime == mtime && entry.size == size {
                        #[cfg(unix)]
                        let mode = if metadata.permissions().mode() & 0o111 != 0 {
                            TreeItemMode::BlobExecutable
                        } else {
                            TreeItemMode::Blob
                        };
                        #[cfg(not(unix))]
                        let mode = TreeItemMode::Blob;
                        items.push(TreeItem::new(mode, entry.hash, file_name));
                        continue;
                    }
                }

                let content = fs::read(&path).map_err(|e| e.to_string())?;
                let blob_hash = object::write_git_object(git_dir, "blob", &content)
                    .map_err(|e| e.to_string())?;

                #[cfg(unix)]
                let mode = if metadata.permissions().mode() & 0o111 != 0 {
                    TreeItemMode::BlobExecutable
                } else {
                    TreeItemMode::Blob
                };
                #[cfg(not(unix))]
                let mode = TreeItemMode::Blob;

                items.push(TreeItem::new(mode, blob_hash, file_name));
            }
        }

        items.sort_by(|a, b| a.name.cmp(&b.name));
        Tree::from_tree_items(items).map_err(|e| e.to_string())
    }

    build_tree_recursive(workdir, git_dir, index, workdir)
}

/// Performs a three-way merge of tree objects.
/// This is a simplified implementation that prefers the stash version in case of conflicts.
fn merge_trees(base: &Tree, head: &Tree, stash: &Tree, git_dir: &Path) -> Result<Tree, String> {
    let base_items = tree::get_tree_files_recursive(base, git_dir, &PathBuf::new())?;
    let mut head_items = tree::get_tree_files_recursive(head, git_dir, &PathBuf::new())?;
    let stash_items = tree::get_tree_files_recursive(stash, git_dir, &PathBuf::new())?;
    let mut conflicts = Vec::new();

    // Iterate through stash changes and apply them to head_items
    for (path, stash_item) in stash_items.iter() {
        let base_item = base_items.get(path);
        let head_item = head_items.get(path);

        match (base_item, head_item) {
            (Some(b), Some(h)) => {
                // File exists in base, head, and stash
                if b.id != h.id && b.id != stash_item.id && h.id != stash_item.id {
                    // Modified in both head and stash differently. CONFLICT!
                    conflicts.push(path.clone());
                    continue; // Skip applying this change
                }

                if b.id != stash_item.id && h.id != stash_item.id {
                    // Modified in stash and potentially in head. Stash wins.
                    head_items.insert(path.clone(), stash_item.clone());
                } else if b.id == h.id && b.id != stash_item.id {
                    // Not modified in head, but modified in stash. Apply stash change.
                    head_items.insert(path.clone(), stash_item.clone());
                }
            }
            (Some(_), None) => {
                // File was deleted in head, but exists in stash. This is a conflict.
                // For stash apply, we restore the file.
                head_items.insert(path.clone(), stash_item.clone());
            }
            (None, Some(_)) => {
                // File added in head, also exists in stash (likely added there too).
                // Stash version takes precedence.
                head_items.insert(path.clone(), stash_item.clone());
            }
            (None, None) => {
                // File is new in stash. Add it.
                head_items.insert(path.clone(), stash_item.clone());
            }
        }
    }

    // Handle deletions: if a file was in base but not in stash, it was deleted.
    for (path, base_item) in base_items.iter() {
        if !stash_items.contains_key(path) {
            // File deleted in stash. Check if it was modified in head.
            if let Some(head_item) = head_items.get(path) {
                if head_item.id != base_item.id {
                    conflicts.push(path.clone());
                    continue;
                }
            }
            head_items.remove(path);
        }
    }

    if !conflicts.is_empty() {
        let error_message = format!(
            "error: Your local changes to the following files would be overwritten by merge:\n  {}\n\
             Please commit your changes or stash them before you merge.",
            conflicts.join("\n  ")
        );
        return Err(error_message);
    }

    let final_items: Vec<TreeItem> = head_items.values().cloned().collect();
    Tree::from_tree_items(final_items).map_err(|e| e.to_string())
}
