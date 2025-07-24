use mercury::hash::SHA1;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::manager::store::{TempStoreArea, TreeStore};

use super::store::ModifiedStore;

/// Color Extraction Macro
#[cfg(debug_assertions)]
macro_rules! color_info {
    ($($arg:tt)*) => {
        println!("[\x1b[34mINFO\x1b[0m] {}", format!($($arg)*));
    };
}

/// Auxiliary function, extracts the `file_name` of Path and
/// converts it to String type.
///
/// Error: When encountering the root directory or other
/// error conditions, the root directory is also returned.
fn path_name_to_string(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/".to_string())
}

/// Auxiliary function, Concatenate path and `old_root_path`
/// to return the real path.
///
/// Error: When new_path is `Path::new("")` or
/// `Path::new("/")`, returns old_root_path
fn get_real_path(path: &Path, old_root_path: &Path) -> PathBuf {
    match path.as_os_str().is_empty() || path.eq(Path::new("/")) {
        // The root path
        true => old_root_path.to_path_buf(),
        // Sub path
        false => old_root_path.join(path),
    }
}

/// Auxiliary function, sort the `tree_items` in the Tree by
/// the first letter.
fn sort_tree_items(tree: &mut Tree) {
    tree.tree_items
        .sort_by(|item1, item2| item1.name.cmp(&item2.name));
}

/// Use `item_data` to update the `tree` and return the tree
/// ID.
fn update_treeitem(tree: &mut Tree, item_data: &TreeItem) -> SHA1 {
    match tree
        .tree_items
        .iter_mut()
        .find(|i| i.name == item_data.name)
    {
        Some(item) => *item = item_data.clone(),
        None => tree.tree_items.push(item_data.clone()),
    }

    sort_tree_items(tree);
    // Rehash
    tree.rehash();
    tree.id
}

/// Read the new Tree into a HashMap
fn build_new_tree_map(
    index_db: &sled::Db,
    old_tree_db: &sled::Db,
    old_root_path: &Path,
) -> sled::Result<HashMap<PathBuf, Tree>> {
    let mut res = HashMap::<PathBuf, Tree>::new();

    for (mut new_path, hash) in index_db.db_list()? {
        // Create the Blob TreeItem.
        let mut sub_item_data = TreeItem {
            mode: TreeItemMode::Blob,
            id: SHA1::from_str(&hash).expect("Hash parsing error in temporary storage area."),
            name: path_name_to_string(&new_path),
        };
        while new_path.pop() {
            // println!("new_path = {}", new_path.display());
            let parent_path = get_real_path(&new_path, old_root_path);
            // println!("parent_path = {}", parent_path.display());
            // println!("old_root_path = {}", old_root_path.display());
            // println!("res = {:?}", old_tree_db.get_bypath(&parent_path));
            // println!("res = {:?}", old_tree_db.get_bypath(&old_root_path));
            let mut sub_item_id = SHA1::default();

            // Check, add and update the Tree into the HashMap
            res.entry(parent_path.clone())
                // The old Tree has been read into the HashMap, update it
                .and_modify(|tree| sub_item_id = update_treeitem(tree, &sub_item_data))
                // The Tree is not exist, check the old_tree_db
                .or_insert_with(|| {
                    let mut tree = match old_tree_db.get_bypath(&parent_path) {
                        // The old Tree exists, update it.
                        Ok(tree) => {
                            #[cfg(debug_assertions)]
                            color_info!("Old Tree: \x1b[1;32m{}\x1b[0m", parent_path.display());
                            tree
                        }
                        // The old Tree is not exist, create it.
                        Err(_) => {
                            #[cfg(debug_assertions)]
                            color_info!("New Tree: \x1b[1;32m{}\x1b[0m", parent_path.display());
                            Tree::from_tree_items(vec![]).unwrap()
                        }
                    };
                    // Update the new TreeItem
                    sub_item_id = update_treeitem(&mut tree, &sub_item_data);

                    tree
                });

            // Create a TreeItem for the current Tree
            // and pass it up.
            sub_item_data = TreeItem {
                mode: TreeItemMode::Tree,
                id: sub_item_id,
                name: path_name_to_string(&parent_path),
            };
        }
        #[cfg(debug_assertions)]
        show_hashmap(&res);
    }

    Ok(res)
}

/// Check `sub_item_data` and perform delete and update on
/// `tree`. If the tree is not empty after the operation,
/// return `Some(tree.id)`, otherwise return `None`.
fn del_treeitem(tree: &mut Tree, sub_item_data: (String, Option<SHA1>)) -> Option<SHA1> {
    let sub_item_name = sub_item_data.0;
    match sub_item_data.1 {
        Some(hash) => match tree.tree_items.iter_mut().find(|i| i.name == sub_item_name) {
            Some(item) => {
                item.id = hash;
            }
            None => panic!("Unexpected index, the rm TreeItem object not found"),
        },
        None => tree.tree_items.retain(|item| item.name != sub_item_name),
    }

    if tree.tree_items.is_empty() {
        None
    } else {
        // Rehash
        tree.rehash();
        Some(tree.id)
    }
}

/// Read the removed files into a HashMap
fn build_removed_tree_map(
    rm_db: &sled::Db,
    old_tree_db: &sled::Db,
    res: &mut HashMap<PathBuf, Tree>,
    old_root_path: &Path,
) -> sled::Result<()> {
    // Use the HashMap<String> to store the Path Tree
    // It is expected that using HashSet to replace Vec<Path> will bring the following benefits:
    //   1. Improved performance
    //   2. Simplified steps
    //   3. Increased operability and flexibility
    for mut new_path in rm_db.path_list()? {
        // Get the TreeItem name.
        let mut sub_item_data: (String, Option<SHA1>) = (path_name_to_string(&new_path), None);

        while new_path.pop() {
            let parent_path = get_real_path(&new_path, old_root_path);

            // Use the `entry` API to avoid multiple lookups
            let entry = res.entry(parent_path.clone());
            // Check and del the Tree into the HashMap
            sub_item_data = (
                path_name_to_string(&parent_path),
                match entry {
                    // The old Tree has been read into the HashMap, update it
                    Entry::Occupied(mut occupied) => {
                        if let Some(hash) = del_treeitem(occupied.get_mut(), sub_item_data) {
                            Some(hash)
                        } else {
                            occupied.remove();
                            None
                        }
                    }
                    // The Tree is not exist, check the old_tree_db
                    Entry::Vacant(vacant) => {
                        #[cfg(debug_assertions)]
                        color_info!("Old Tree: \x1b[1;32m{}\x1b[0m", parent_path.display());
                        let mut tree = old_tree_db
                            .get_bypath(&parent_path)
                            .expect("Unexpected index, the rm Tree object not found");
                        if let Some(hash) = del_treeitem(&mut tree, sub_item_data) {
                            let _ = vacant.insert(tree);
                            Some(hash)
                        } else {
                            None
                        }
                    }
                },
            );
        }
    }

    Ok(())
}

/// This function is used to format and print HashMap<PathBuf, Tree>
#[cfg(debug_assertions)]
fn show_hashmap(hashmap: &HashMap<PathBuf, Tree>) {
    for (tmp1, tmp2) in hashmap.iter() {
        print!("  {} -> {} :", tmp1.display(), tmp2.id,);
        let mut tmp2 = tmp2.clone();
        tmp2.rehash();
        for tmp3 in tmp2.tree_items.iter() {
            print!(
                "{{\n\tmode: {},\n\tid: {},\n\tname: {},\n}}",
                tmp3.mode,
                tmp3.id._to_string(),
                tmp3.name
            )
        }
    }
}

/// This function is the core function of the commit operation.
///
/// It can use the data in the temporary storage area and the
/// deletion records of the whiteout file to update the old
/// version tree, write it to the new database, and return the
/// Hash of the main Tree.
pub fn commit_core(
    // Includes the old tree.db containing
    //the tree structure of the previous
    // version and the new tree.db.
    //
    // tree_dbs = (old_tree_db, new_tree_db)
    tree_dbs: (&sled::Db, &sled::Db),
    temp_store_area: &TempStoreArea, // The temporary storage area.
    old_root_path: &Path,            // The path of the main Tree in tree.db.
) -> sled::Result<SHA1> {
    // To prevent the Remove operation from affecting the
    // Vec<TreeItem> of the main Tree, we now change it to performing
    // the Remove operation first, then extracting the main Tree
    // and performing the next operation.
    println!("\x1b[34m[PART1]\x1b[0m");
    // The temporary storage area Db contains the files that need to be added.
    let index_db = &temp_store_area.index_db;
    // The Db who storing whiteout files.
    let rm_db = &temp_store_area.rm_db;

    let old_tree_db = tree_dbs.0;
    let new_tree_db = tree_dbs.1;

    let mut hashmap = build_new_tree_map(index_db, old_tree_db, old_root_path)?;
    #[cfg(debug_assertions)]
    show_hashmap(&hashmap);

    println!("\x1b[34m[PART2]\x1b[0m");
    build_removed_tree_map(rm_db, old_tree_db, &mut hashmap, old_root_path)?;
    #[cfg(debug_assertions)]
    show_hashmap(&hashmap);

    // Insert the new Tree into the Db.
    //
    // Since the insert of sled::Db is an
    // overwrite operation, we can update it
    // with confidence.
    let mut batch = sled::Batch::default();
    let config = bincode::config::standard();
    for (path, tree) in hashmap.iter() {
        batch.insert(
            path.to_string_lossy().into_owned().as_str(),
            bincode::encode_to_vec(tree, config).unwrap(),
        );
    }
    new_tree_db.apply_batch(batch)?;

    Ok(hashmap.remove(old_root_path).unwrap().id)
}
