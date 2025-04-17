use mercury::hash::SHA1;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::manager::store::TreeStore;

use super::store::ModifiedStore;

fn update_treeitem(current: &mut Tree, name: &str, hash: SHA1, item_mode: TreeItemMode) {
    match current.tree_items.iter_mut().find(|i| i.name == name) {
        Some(item) => {
            item.id = hash;
            item.mode = item_mode;
        }
        None => current.tree_items.push(TreeItem {
            mode: item_mode,
            id: hash,
            name: name.to_string(),
        }),
    }
}

/// This function is used to recursively use a path to update
/// its corresponding TreeItem and all its parent Tree objects.
fn update_tree_with_index_path(
    commit_db: &sled::Db,
    current: &mut Tree,
    index_path: &Path,
    blob_hash: SHA1,
) -> sled::Result<()> {
    let mut components = index_path.components().peekable();
    let mut child_path = PathBuf::new();

    // Loop to iterate over the path.
    if let Some(comp) = components.next() {
        let name = comp.as_os_str().to_string_lossy().to_string();

        // Since the temporary storage area stores files, the
        // last item of the iterator is the file name.
        if components.peek().is_none() {
            println!("        [\x1b[34mINFO\x1b[0m] Last one.");
            println!(
                "    [\x1b[33mDEBUG\x1b[0m] comp = {}",
                comp.as_os_str().to_string_lossy()
            );

            // If the TreeItem already exists, update its Hash
            // and TreeItemMode, otherwise create a new one.
            update_treeitem(current, &name, blob_hash, TreeItemMode::Blob);
        } else {
            // Extract child path.
            child_path.push(comp);
            println!(
                "    [\x1b[33mDEBUG\x1b[0m] mut child_path = {}",
                child_path.display()
            );

            // Extract the subtree from the database using the
            // subpath, creating a new one if it does not exist.
            let mut subtree = commit_db.get_bypath(&child_path).unwrap_or(Tree {
                id: SHA1::default(),
                tree_items: Vec::new(),
            });

            // Recursively call the update_tree_with_blob_path
            // function to enter the next level of directory.
            update_tree_with_index_path(
                commit_db,
                &mut subtree,
                index_path.strip_prefix(&child_path).unwrap(),
                blob_hash,
            )?;
            println!(
                "    [\x1b[33mDEBUG\x1b[0m] {}.tree_items.len() = {}",
                index_path.display(),
                subtree.tree_items.len()
            );
            subtree
                .tree_items
                .iter()
                .for_each(|tmp| print!("\t{}", tmp.name));
            println!();

            // Use the rehash function to update the Hash value
            // of the subtree.
            subtree.rehash();

            // If the TreeItem already exists, update its Hash,
            // otherwise create a new one.
            update_treeitem(current, &name, subtree.id, TreeItemMode::Tree);

            // Write the new Tree to the Db.
            commit_db.insert_tree(child_path.to_path_buf(), subtree);
        }
    }

    Ok(())
}

/// This function is used to delete the whiteout file and recursively
/// update the main Tree.
fn update_tree_with_rm_path(commit_db: &sled::Db, parent_path: &Path) -> sled::Result<()> {
    let mut name = parent_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    while let Some(parent_path) = parent_path.parent() {
        // Extract the parent Tree object and filter out the TreeItem
        // corresponding to the current path.
        let mut parent_tree = commit_db.get_bypath(parent_path)?;
        parent_tree.tree_items.retain(|item| item.name != name);

        // When the screening is complete, update name immediately.
        name = parent_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        // If the current Tree object no longer contains any TreeItem,
        // the current Tree will be deleted directly and the next loop
        // will be entered.
        if parent_tree.tree_items.is_empty() {
            commit_db.remove(parent_path.to_string_lossy().as_bytes())?;
            continue;
        }

        // Otherwise update the Tree object and write to Db.
        parent_tree.rehash();
        commit_db.insert_tree(parent_path.to_path_buf(), parent_tree);
    }

    Ok(())
}

/// This function is a further encapsulation of "updating
/// the Tree object using path->hash key-value pairs".
fn update_with_index(index_db: &sled::Db, commit_db: &sled::Db, root_tree: &mut Tree) -> sled::Result<()> {
    // Traverse all key-value pairs in the library and sort them
    // by length to ensure depth priority.
    let mut index_staged: Vec<_> = index_db.iter().collect::<Result<Vec<_>, _>>()?;
    index_staged.sort_by_key(|(k, _)| k.len());
    let index_staged = index_staged;

    println!("\x1b[34m[PART2]\x1b[0m");
    // Call the update_tree_with_blob_path function for all
    // (path, hash) tuples to update the main Tree.
    for (key_bytes, hash_bytes) in index_staged.iter() {
        let index_path: PathBuf = PathBuf::from(String::from_utf8_lossy(key_bytes).to_string());
        let blob_hash: SHA1 =
            SHA1::from_str(String::from_utf8(hash_bytes.to_vec()).unwrap().as_str()).unwrap();

        println!(
            "[\x1b[33mDEBUG\x1b[0m] index_path = {}",
            index_path.display()
        );
        println!(
            "[\x1b[33mDEBUG\x1b[0m] blob_hash = {}",
            blob_hash._to_string()
        );

        update_tree_with_index_path(commit_db, root_tree, &index_path, blob_hash)?;
    }

    Ok(())
}

/// This function is a further encapsulation of "updating
/// the Tree object using the whiteout path".
fn update_with_rm(rm_db: &sled::Db, commit_db: &sled::Db) -> sled::Result<()> {
    // List all whiteout files.
    let rm_staged = rm_db.path_list()?;

    println!("\x1b[34m[PART4]\x1b[0m");
    // Execute the update_tree_with_rm_path function for all
    // whiteout files.
    for rm_path in rm_staged.iter() {
        update_tree_with_rm_path(commit_db, rm_path)?;
    }

    Ok(())
}

/// This function is the core function of the commit operation.
///
/// It can extract the data in the staging area and the removal
/// records of the whiteout files, and use them to update the old
/// version tree.
pub fn commit_core(
    commit_db: &sled::Db, // A copy of tree.db can be modified directly.
    index_db: &sled::Db,  // The temporary storage area Db contains the files that need to be added.
    rm_db: &sled::Db,     // The Db who storing whiteout files.
    old_root_path: &Path, // The path of the main Tree in tree.db.
) -> sled::Result<SHA1> {
    // To prevent the Remove operation from affecting the
    // Vec<TreeItem> of the main Tree, we now change it to performing
    // the Remove operation first, then extracting the main Tree
    // and performing the next operation.
    println!("\x1b[34m[PART1]\x1b[0m");
    update_with_rm(rm_db, commit_db)?;

    println!("\x1b[34m[PART2]\x1b[0m");
    // Get the root Tree.
    let mut root_tree = commit_db
        .get_bypath(old_root_path)
        .expect("Old root tree not found");

    println!("\x1b[34m[PART3]\x1b[0m");
    update_with_index(index_db, commit_db, &mut root_tree)?;

    println!("\x1b[34m[PART4]\x1b[0m");
    // Update the main Tree's Hash
    root_tree.rehash();
    let res = root_tree.id;

    println!("\x1b[34m[PART5]\x1b[0m");
    // Write the new Tree to the Db.
    commit_db.insert_tree(old_root_path.to_path_buf(), root_tree);
    commit_db.flush()?;

    println!("\x1b[34m[OK]\x1b[0m");
    Ok(res)
}
