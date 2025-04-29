use libc::{self, stat};
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use mercury::internal::object::types::ObjectType;
use std::collections::HashMap;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::vec;

use crate::manager::store::TreeStore;

fn collect_paths<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                paths.push(path.to_path_buf());
                paths.extend(collect_paths(&path));
            } else {
                paths.push(path.to_path_buf());
            }
        }
    }

    paths
}

// is a file or file node is whiteout file?
pub fn is_whiteout_inode<P: AsRef<Path>>(path: P) -> bool {
    let c_path =
        CString::new(path.as_ref().to_string_lossy().as_bytes()).expect("CString::new failed");
    let mut stat_buf: stat = unsafe { std::mem::zeroed() };
    // Call the stat function from libc to get the file status
    let result = unsafe { libc::stat(c_path.as_ptr(), &mut stat_buf) };

    // Check if the stat call was successful
    if result == 0 {
        // Check if the file mode is a character device
        return stat_buf.st_mode == (libc::S_IFCHR | 0o777);
    }

    false
}
/// the output tree in hashmap , which hash value is not computed.
#[allow(unused)]
pub fn diff(
    lower: PathBuf,
    upper: PathBuf,
    dbpath: &str,
    monopath: PathBuf,
) -> std::io::Result<(HashMap<PathBuf, Tree>, Vec<Blob>)> {
    let db = sled::open(dbpath)?;
    let upper_changes = collect_paths(&lower);
    let _root_len = lower.clone();
    let mut map = HashMap::<PathBuf, Tree>::new();
    let root_tree = db.get_bypath(&monopath)?; // BUG: wrong path :"lower". the store in db is not real path .
                                               // dont forget get tree below
    map.insert(lower.clone(), root_tree);
    let mut blobs = Vec::<Blob>::new();

    for node in upper_changes {
        if node.is_dir() {
            let new_path = monopath.clone().join(node.strip_prefix(&lower).unwrap());
            let node_tree = db.get_bypath(&new_path)?;
            map.insert(node.clone(), node_tree);
            //db.get_bypath(path);
            //ap.insert(node.clone(), node_tree);
        }
        let p = node.parent().unwrap().to_path_buf();
        let t = map.get_mut(&p).unwrap();

        let mut is_new_file = true;
        // look up the parent tree node to delete/change .
        let mut i = 0;
        while i < t.tree_items.len() {
            let item = &mut t.tree_items[i];
            if item
                .name
                .eq(node.file_name().unwrap().to_string_lossy().as_ref())
            {
                is_new_file = false;
                //delete .
                if is_whiteout_inode(&node) {
                    t.tree_items.remove(i);
                } else if node.is_dir() {
                    //pass
                } else {
                    // changed
                    // node is a changed ()blob.
                    // just compute the NEW hash first.
                    let content = std::fs::read(&node)?;
                    item.id = SHA1::from_type_and_data(ObjectType::Blob, &content);
                    blobs.push(Blob {
                        id: item.id,
                        data: content,
                    });
                }
                break;
            }
            i += 1;
        }
        // if new file add item.
        let new_name = node
            .file_name()
            .unwrap()
            .to_string_lossy()
            .as_ref()
            .to_owned();
        if is_new_file {
            //is a fiel or a dictionary?
            if node.is_dir() {
                // is dictionary.
                t.tree_items.push(TreeItem {
                    mode: TreeItemMode::Tree,
                    id: SHA1::default(),
                    name: new_name,
                });
            } else {
                //is a file.
                let content = std::fs::read(&node)?;
                let hash = SHA1::from_type_and_data(ObjectType::Blob, &content);
                blobs.push(Blob {
                    id: hash,
                    data: content,
                });
                t.tree_items.push(TreeItem {
                    mode: TreeItemMode::Blob,
                    id: hash,
                    name: new_name,
                });
            }
        }
    }
    Ok((map, blobs))
}

pub fn change(
    real_path: PathBuf,
    tree_path: PathBuf,
    trees: &mut Vec<Tree>,
    blobs: &mut Vec<Blob>,
    db: &sled::Db,
) -> Tree {
    let mut tree;
    let root_tree = db.get_bypath(&tree_path);
    if let Ok(root_tree) = root_tree {
        // exit dictionry
        println!("exit tree:{:?}", tree_path);
        tree = root_tree;
    } else {
        // there is a new dictionary.
        println!("new tree:{:?}", tree_path);
        tree = Tree {
            id: SHA1::default(),
            tree_items: vec![],
        };
    }

    let entries = std::fs::read_dir(real_path).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
            .to_string();
        let mut new = true;
        let mut i = 0;
        while i < tree.tree_items.len() {
            let item: &mut TreeItem = tree.tree_items.get_mut(i).unwrap();
            if item.name.eq(&name) {
                if is_whiteout_inode(&path) {
                    println!("change: delete {}", item.name);
                    // delete..
                    let _ = item; //drop
                    tree.tree_items.remove(i);
                } else if path.is_dir() {
                    let new_tree_path = tree_path.join(path.file_name().unwrap());
                    let new_tree = change(path.clone(), new_tree_path, trees, blobs, db);

                    // change the hash value & push tree .
                    item.id = new_tree.id;
                    trees.push(new_tree);
                    println!("change: changed tree {}", item.name);
                } else {
                    println!("change: changed file {}", item.name);
                    let content = std::fs::read(&path).unwrap();
                    let hash = SHA1::from_type_and_data(ObjectType::Blob, &content);
                    blobs.push(Blob {
                        id: hash,
                        data: content,
                    });
                    item.id = hash; // change fiel hash .
                }
                new = false;
                break;
            }
            i += 1;
        }
        if new {
            // a new file or dictionary
            if path.is_dir() {
                println!("change: new tree  {:?}", name);
                let new_tree_path = tree_path.join(path.file_name().unwrap());
                let new_tree = change(path.clone(), new_tree_path, trees, blobs, db);

                //add a new item for this tree. and push tree.
                tree.tree_items.push(TreeItem {
                    mode: TreeItemMode::Tree,
                    id: new_tree.id,
                    name,
                });
                trees.push(new_tree);
            } else {
                println!("change: new file  {}", name);
                let content = std::fs::read(&path).unwrap();
                let hash = SHA1::from_type_and_data(ObjectType::Blob, &content);
                blobs.push(Blob {
                    id: hash,
                    data: content,
                });
                tree.tree_items.push(TreeItem {
                    mode: TreeItemMode::Blob,
                    id: hash,
                    name,
                });
            }
        }
    }
    tree.rehash(); //Re compute the hash value .
    tree
}

#[cfg(test)]
mod tests {

    use std::fs::File;

    use super::*;

    #[test]
    fn test_collect_paths_nested_directories() {
        let temp_dir = "/home/luxian/code/mega/scorpio/src";
        let paths = collect_paths(temp_dir);
        for p in paths {
            println!("{:?}", p);
        }
    }

    #[test]
    fn test_is_whiteout_inode() {
        // Create a temporary file
        let temp_file_path = "temp_file.txt";
        let _file = File::create(temp_file_path).expect("Unable to create file");

        // Check if the file is a character device
        assert!(!is_whiteout_inode(temp_file_path));

        // Clean up the temporary file
        std::fs::remove_file(temp_file_path).expect("Unable to delete file");
    }
}
