use mercury::hash::SHA1;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use mercury::internal::object::types::ObjectType;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::manager::diff::is_whiteout_inode;
use crate::manager::store::BlobFsStore;

/// Currently, there is no good way to directly convert the
/// traversal results of WalkDir into the existing Tree and
/// TreeItem structures.
///
/// This version uses HashMap as a bridge.
#[derive(Debug, Clone)]
enum TmpTree {
    Blob { hash: SHA1, executable: bool },
    // Binary Tree
    Tree { children: HashMap<String, TmpTree> },
}

/// This function is used to determine whether
/// a file is executable
fn get_blob_mode(metadata: &std::fs::Metadata) -> TreeItemMode {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match metadata.permissions().mode() & 0o111 != 0 {
            true => TreeItemMode::BlobExecutable,
            false => TreeItemMode::Blob,
        }
    }
    #[cfg(not(unix))]
    {
        TreeItemMode::Blob // fallback
    }
}

fn insert_path(
    tree: &mut TmpTree,
    path: &Path,
    hash: SHA1,
    mode: TreeItemMode,
    signed_paths: &mut HashSet<String>,
) {
    let mut current = tree;
    let mut path_components = path.components().peekable();
    let mut tmp_path = PathBuf::from("");

    while let Some(component) = path_components.next() {
        tmp_path.push(component);
        let name = tmp_path.as_os_str().to_string_lossy().to_string();
        println!(
            "        [\x1b[33mDEBUG\x1b[0m] tmp_path = {}",
            tmp_path.display()
        );

        current = match current {
            TmpTree::Tree { children } => match path_components.peek().is_none() {
                true => {
                    println!("        [\x1b[34mINFO\x1b[0m] Last one.");
                    let _ = children.insert(
                        name,
                        match mode {
                            TreeItemMode::Blob | TreeItemMode::BlobExecutable => TmpTree::Blob {
                                hash,
                                executable: mode.eq(&TreeItemMode::BlobExecutable),
                            },
                            _ => panic!("Unsupported TreeItemMode in insert_path"),
                        },
                    );
                    return;
                }
                false => {
                    signed_paths.insert(name.clone());
                    children
                        .entry(name.clone())
                        .or_insert_with(|| TmpTree::Tree {
                            children: HashMap::new(),
                        })
                }
            },
            _ => panic!("Path collision with file and directory"),
        };
    }
}

/// Use the root path and convert it into a binary tree
fn build_hashmap_from_root_path(
    root: &str,
    work_dir: &PathBuf,
    rm_batch: &mut sled::Batch,
    signed_paths: &mut HashSet<String>,
) -> Result<TmpTree, Box<dyn Error>> {
    println!("    [\x1b[33mDEBUG\x1b[0m] root = {root}");
    println!(
        "    [\x1b[33mDEBUG\x1b[0m] work_dir.display = {}",
        work_dir.display()
    );
    let mut root_node = TmpTree::Tree {
        children: HashMap::new(),
    };

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let real_path = entry.path();
        let path = real_path.strip_prefix(root)?;

        println!(
            "    [\x1b[33mDEBUG\x1b[0m] real_path.display = {}",
            real_path.display()
        );
        println!(
            "    [\x1b[33mDEBUG\x1b[0m] path.display = {}",
            path.display()
        );

        match is_whiteout_inode(real_path) {
            true => {
                let key = path.to_string_lossy().to_string();
                println!("    [\x1b[34mINFO\x1b[0m] whiteout_inode: {key}");
                rm_batch.insert(key.as_bytes(), b"");
            }
            false => {
                let content = std::fs::read(real_path)?;
                let hash = SHA1::from_type_and_data(ObjectType::Blob, &content);
                work_dir.add_blob_to_hash(&hash._to_string(), &content)?;

                let metadata = entry.metadata()?;
                let mode = get_blob_mode(&metadata);

                println!("    [\x1b[34mINFO\x1b[0m] Start loop insertion paths.");
                insert_path(&mut root_node, path, hash, mode, signed_paths);
                println!("    [\x1b[34mINFO\x1b[0m] Done.");
            }
        }
    }

    Ok(root_node)
}

/// Thanks to the fact that the index of Tree in Db is
/// stored as a path, we can easily determine whether
/// each Tree object still exists.
fn remove_old_record(
    db: &sled::Db,
    batch: &mut sled::Batch,
    signed_paths: &HashSet<String>,
) -> sled::Result<()> {
    db.iter().try_for_each(|result| match result {
        Ok((path, _)) => {
            let path_string = String::from_utf8_lossy(&path).to_string();
            match signed_paths.contains(&path_string) {
                true => (),
                false => {
                    println!("    [\x1b[33mDEBUG\x1b[0m] Remove {path_string}");
                    batch.remove(path)
                }
            }

            Ok(())
        }
        Err(e) => Err(e),
    })
}

/// Convert the binary tree into a Tree and TreeItem structure
fn flatten_tree_with_batch(
    node: &TmpTree,
    current_path: &Path,
    out_map: &mut HashMap<PathBuf, Tree>,
) -> SHA1 {
    match node {
        TmpTree::Tree { children } => {
            let mut tree_items = Vec::new();

            for (name, child) in children {
                let child_path = current_path.join(name);
                println!(
                    "        [\x1b[33mDEBUG\x1b[0m] current_path = {}",
                    current_path.display()
                );
                println!(
                    "        [\x1b[33mDEBUG\x1b[0m] child_path = {}",
                    child_path.display()
                );

                match child {
                    TmpTree::Blob { hash, executable } => {
                        tree_items.push(TreeItem {
                            mode: match *executable {
                                true => TreeItemMode::BlobExecutable,
                                false => TreeItemMode::Blob,
                            },
                            id: *hash,
                            name: name.clone(),
                        });
                    }
                    TmpTree::Tree { .. } => {
                        // I can't think of a better way except recursion.
                        let child_hash = flatten_tree_with_batch(child, &child_path, out_map);
                        tree_items.push(TreeItem {
                            mode: TreeItemMode::Tree,
                            id: child_hash,
                            name: name.clone(),
                        });
                    }
                }
            }

            // Equivalent to calling the rehash() function after creation.
            let mut data = Vec::new();
            for item in &tree_items {
                data.extend_from_slice(item.to_data().as_slice());
            }

            let id = SHA1::from_type_and_data(ObjectType::Tree, &data);
            let tree = Tree { id, tree_items };

            println!(
                "        [\x1b[34mINFO\x1b[0m] Write {} to a batch",
                current_path.display()
            );
            out_map.insert(current_path.to_path_buf(), tree);

            id
        }

        _ => panic!("flatten_tree_with_batch called on non-directory node"),
    }
}

fn write_all_trees_to_batch(
    root_node: &TmpTree,
    batch: &mut sled::Batch,
) -> Result<(), Box<dyn Error>> {
    let mut tree_map = HashMap::new();
    let _ = flatten_tree_with_batch(root_node, Path::new(""), &mut tree_map);

    println!("    [\x1b[34mINFO\x1b[0m] Write Tree objects to a batch");
    for (path, tree) in tree_map.iter() {
        let key = path.to_str().unwrap();
        let value = bincode::serialize(tree)?;
        batch.insert(key, value)
    }

    Ok(())
}

/// This function should not make any changes to the existing Tree structure,
/// and should only make changes during the Commit operation.
///
/// This version uses the HashMap structure to store and search Tree objects,
/// thus avoiding the double pointer problem.
///
/// Of course, I also provide a list_paths API, which only returns a vector of
/// PathBuf stored in the database. That is another solution.
///
/// sled::Db also provides a get() function, but I am not sure about its
/// performance and security, and it has too many restrictions.
pub fn add_and_del(
    real_path: PathBuf,
    work_dir: PathBuf,
    index_db: &sled::Db,
    rm_db: &sled::Db,
) -> Result<(), Box<dyn Error>> {
    // Using batch processing to simplify I/O operations
    // and reduce disk consumption.
    let mut index_batch_space = sled::Batch::default();
    let mut rm_batch_space = sled::Batch::default();
    let mut signed_paths = HashSet::new();

    println!("\x1b[32m[PART1]\x1b[0m Build the HashMap");
    let root_node = build_hashmap_from_root_path(
        real_path.to_string_lossy().as_ref(),
        &work_dir,
        &mut rm_batch_space,
        &mut signed_paths,
    )?;

    println!("\x1b[32m[PART2]\x1b[0m Remove invalid paths");
    // Changing Mutability
    let signed_paths = signed_paths;
    remove_old_record(index_db, &mut index_batch_space, &signed_paths)?;

    println!("\x1b[32m[PART3]\x1b[0m Convert HashMap to Tree structure and write to batch");
    write_all_trees_to_batch(&root_node, &mut index_batch_space)?;

    println!("\x1b[32m[PART4]\x1b[0m Excute the batch");
    index_db.apply_batch(index_batch_space)?;
    rm_db.apply_batch(rm_batch_space)?;

    println!("\x1b[32m[DONE]\x1b[0m");
    Ok(())
}
