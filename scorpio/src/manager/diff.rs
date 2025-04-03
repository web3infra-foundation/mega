use libc::{self, stat};
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use mercury::internal::object::types::ObjectType;
use std::collections::HashMap;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::vec;

use crate::manager::store::{ModifiedKVStore, StorageSpace, TreeStore};

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
fn is_whiteout_inode<P: AsRef<Path>>(path: P) -> bool {
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
) -> (HashMap<PathBuf, Tree>, Vec<Blob>) {
    let db = sled::open(dbpath).unwrap();
    let upper_changes = collect_paths(&lower);
    let _root_len = lower.clone();
    let mut map = HashMap::<PathBuf, Tree>::new();
    let root_tree = db.get_bypath(monopath.clone()).unwrap(); // BUG: wrong path :"lower". the store in db is not real path .
                                                              // dont forget get tree below
    map.insert(lower.clone(), root_tree);
    let mut blobs = Vec::<Blob>::new();

    for node in upper_changes {
        if node.is_dir() {
            let new_path = monopath.clone().join(node.strip_prefix(&lower).unwrap());
            let node_tree = db.get_bypath(new_path).unwrap();
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
                    let content = std::fs::read(&node).unwrap();
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
                let content = std::fs::read(&node).unwrap();
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
    (map, blobs)
}

pub fn change(
    real_path: PathBuf,
    tree_path: PathBuf,
    trees: &mut Vec<Tree>,
    blobs: &mut Vec<Blob>,
    db: &sled::Db,
) -> Tree {
    let mut tree;
    let root_tree = db.get_bypath(tree_path.clone());
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

/// Since custom errors are cumbersome and it is not
/// easy to implement error type conversion of
/// Box<dyn std::error::Error>, a feature is defined
/// here to convert std::io::Error to the error type
/// of Box<dyn std::error::Error> and attach an error
/// message.
pub trait ErrorTurn<T> {
    fn box_from_io_with_msg(self, message: &str) -> Result<T, Box<dyn std::error::Error>>;
}
impl<T> ErrorTurn<T> for Result<T, std::io::Error> {
    fn box_from_io_with_msg(self, message: &str) -> Result<T, Box<dyn std::error::Error>> {
        match self {
            Ok(res) => Ok(res),
            Err(e) => {
                eprintln!("{message}: {}", e);
                Err(Box::from(e))
            }
        }
    }
}
impl<T> ErrorTurn<T> for Result<T, sled::Error> {
    fn box_from_io_with_msg(self, message: &str) -> Result<T, Box<dyn std::error::Error>> {
        match self {
            Ok(res) => Ok(res),
            Err(e) => {
                eprintln!("{message}: {}", e);
                Err(Box::from(e))
            }
        }
    }
}

#[inline(always)]
fn read_dir_to_vec(path: &PathBuf) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_dir(path)
        .box_from_io_with_msg("Failed to read directory")?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<PathBuf>>())
}

#[inline(always)]
fn get_file_hash(path: &PathBuf) -> String {
    let content: Vec<u8> = std::fs::read(path).unwrap();
    SHA1::from_type_and_data(ObjectType::Blob, &content)._to_string()
}

/// A wrapper add function for adding new files to a blob object.
fn add_blob(
    real_path: &PathBuf,
    work_dir: &StorageSpace,
    batch: &mut StorageSpace,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("change: Adding file: {}", real_path.display());
    let content = std::fs::read(real_path).unwrap();
    let new_hash = SHA1::from_type_and_data(ObjectType::Blob, &content)._to_string();

    // Add the Hash to Batch.
    print!("Adding hash to path...");
    batch.bat_add_kv(real_path, &new_hash);
    println!("Done.");

    // Add the Blobs to Objects.
    print!("Adding blob to hash...");
    work_dir
        .add_blob_to_hash(&new_hash, &content)
        .box_from_io_with_msg("add_blob: Add blob to hash failed.")?;
    println!("Done.");

    Ok(())
}

/// An encapsulated update function used to update the file
/// modification content and the key value of the Tree object
fn update_hash(
    real_path: &PathBuf,
    work_dir: &StorageSpace,
    db_map: &HashMap<PathBuf, String>,
    batch: &mut StorageSpace,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("change: Updating file: {}", real_path.display());
    let content = std::fs::read(real_path).unwrap();
    let new_hash = SHA1::from_type_and_data(ObjectType::Blob, &content)._to_string();
    let old_hash = db_map.get(real_path).unwrap();

    // Update the Hash in Batch.
    print!("Updating hash to path...");
    batch.bat_add_kv(real_path, &new_hash);
    println!("Done.");

    // Del the Blob in Objects.
    print!("Deling blob by hash...");
    work_dir
        .del_blob_by_hash(old_hash)
        .box_from_io_with_msg("update_blob: Del blob to hash failed.")?;
    println!("Done.");

    // Add the new Blob to Objects.
    print!("Adding blob to hash...");
    work_dir
        .add_blob_to_hash(&new_hash, &content)
        .box_from_io_with_msg("update_blob: Add blob to hash failed.")?;
    println!("Done.");

    Ok(())
}

/// A wrapper deletion function used to delete the file
/// corresponding to the target path from the Tree object
/// and Blob object.
///
/// If the original method is used, it will cause conflicts
/// in the Blob objects of multiple files with different
/// paths but the same content. When the Blob of one of
/// the files is deleted, the delete_blob operation of other
/// files will report an error std::io::ErrorKind::NotFound.
///
/// Now it is changed to use two separate vectors to store
/// Path and Hash, thus avoiding file operation conflicts.
fn delete_blob(
    path_vec: &Vec<PathBuf>,
    work_dir: &StorageSpace,
    hash_vec: &Vec<String>,
    batch: &mut StorageSpace,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Deling hash by path...");
    path_vec.iter().for_each(|path| {
        println!("change: Deleting file: {}", path.display());
        // Del the Hash in Db.
        let _ = batch.bat_del_kv(path);
    });
    println!("Done.");

    print!("Deling blob by hash...");
    let res = hash_vec
        .iter()
        .filter_map(|hash| match hash.as_str() {
            // This avoids secondary deletion of WhiteOut files.
            StorageSpace::WHITEOUT_FLAG => None,
            _ => Some(
                work_dir
                    .del_blob_by_hash(hash)
                    .box_from_io_with_msg("delete_blob: Del blob failed."),
            ),
        })
        .collect::<Result<(), Box<dyn std::error::Error>>>();
    println!("Done.");

    res
}

/// A wrapper add function for adding whiteout files to the db.
fn add_whiteout_inode(
    path: &PathBuf,
    batch: &mut StorageSpace,
    white_out_vec: &Vec<PathBuf>,
) {
    match white_out_vec.iter()
        .find(|&tmp_path| tmp_path.eq(path))
    {
        Some(tmp_path) => println!("This WhiteOut {} has allready added.", tmp_path.display()),
        None => {
            println!("change: WhiteOut file: {}", path.display());
            batch.bat_add_whiteout(path);
        }
    }
}

/// This function dosn't check the input path, so if you call it outside the
/// mono_add() function, be careful the directory injection vulnerability.
///
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
) -> Result<(), Box<dyn std::error::Error>> {
    let db_space: StorageSpace = StorageSpace::SledDb(index_db.clone());
    let path_space: StorageSpace = StorageSpace::BlobFs(work_dir);
    // Using batch processing to simplify I/O operations
    // and reduce disk consumption.
    let mut batch_space: StorageSpace = StorageSpace::SledBat(sled::Batch::default());

    let index_db_map: HashMap<PathBuf, String> = db_space
        .list_db()
        .box_from_io_with_msg("Failed to get HashMap from index DB")?;
    // One problem with using traditional Vec for delete_blob is that when
    // multiple paths correspond to the same hash value, multiple conflicts
    // and errors will occur. See the description of the delete_blob function
    // for details. We tried to introduce an auxiliary database for counting,
    // but abandoned this solution due to the difficulty of API decoupling.
    //
    // This version introduces a counting HashMap to count the number of
    // hash value repetitions. If the number of repetitions is zero, call
    // delete_blob to delete.
    let mut count_db_map: HashMap<String, usize> = db_space
        .list_values()
        .box_from_io_with_msg("Failed to get HashMap from index DB")?;
    let entries: Vec<PathBuf> = match real_path.is_dir() {
        true => read_dir_to_vec(&real_path)?,
        false => vec![real_path.clone()],
    };
    let mut path_vec: Vec<PathBuf> = db_space.list_keys()?;
    let white_out_vec: Vec<PathBuf> = db_space.list_whiteout_file()?;

    /// There are three options here:
    /// 1. One is to use function recursion, which is the
    ///  most conventional;
    /// 2. The second is to use is_dir() judgment similar
    ///  to the previous version, which is the most troublesome;
    /// 3. The last one is to use closure recursion, which
    ///  is an experimental method.
    ///
    /// I prefer the last one.
    ///
    /// Thanks to the hierarchical structure of OverlayFs,
    /// we can now get all the changed files directly from
    /// the Upper folder without having to read from the
    /// mount point using the FUSE system. This saves a lot
    /// of work.
    struct Space<'s> {
        space_01: &'s dyn Fn(
            &Space,
            &Vec<PathBuf>,
            &mut Vec<PathBuf>,
            &mut StorageSpace,
        ) -> Result<(), Box<dyn std::error::Error>>,
    }
    let main_closure = Space {
        space_01: &|main_closure, entries, path_vec, batch| {
            // It will shut down when an element in entries is Err.
            // while let Some(entry) = entries.next() {
            for path in entries {
                // std::thread::sleep(std::time::Duration::new(5, 0));
                match path.is_dir() {
                    // If the path is a directory, recursively call the
                    // closure.
                    true => {
                        let new_entries: Vec<PathBuf> = read_dir_to_vec(path)?;
                        (main_closure.space_01)(main_closure, &new_entries, path_vec, batch)?
                    }
                    // If a file, check the HashMap.
                    false => match is_whiteout_inode(path) {
                        // If a discarded original file, Use special flags
                        // to add it to the database.
                        true => add_whiteout_inode(path, batch, &white_out_vec),
                        // If not, check if the HashMap is empty.
                        false => match index_db_map.is_empty() {
                            // If the HashMap is empty, create blobs object
                            // and update the db.
                            true => add_blob(path, &path_space, batch)?,
                            // If not, check if the path is a whiteout inode.
                            false => match index_db_map.get(path) {
                                // If the path exists in the HashMap, check
                                // the hash to see if it has been modified.
                                Some(hash) => {
                                    let index =
                                        path_vec.iter().position(|tmp| tmp == path).unwrap();
                                    path_vec.remove(index);
                                    match get_file_hash(path).eq(hash) {
                                        // Already latest.
                                        true => (),
                                        // Update the file record.
                                        false => {
                                            update_hash(path, &path_space, &index_db_map, batch)?
                                        }
                                    }
                                }
                                // If it does not exist, create create blobs
                                // object and update the db.
                                None => add_blob(path, &path_space, batch)?,
                            },
                        },
                    },
                }
            }
            Ok(())
        },
    };

    (main_closure.space_01)(&main_closure, &entries, &mut path_vec, &mut batch_space)?;

    // Changing Mutability
    let path_vec: Vec<PathBuf> = path_vec;
    let mut db_map: HashMap<PathBuf, String> = index_db_map;

    let hash_vec: Vec<String> = path_vec
        .iter()
        .map(|path| db_map.remove(path).unwrap())
        .filter_map(|hash| {
            let count = count_db_map.entry(hash.clone()).or_insert(0);
            *count -= 1;
            match count {
                0 => Some(hash),
                _ => None,
            }
        })
        .collect::<Vec<String>>();

    delete_blob(&path_vec, &path_space, &hash_vec, &mut batch_space)?;

    let batch: sled::Batch = batch_space.try_to_bat().unwrap().to_owned();
    index_db
        .apply_batch(batch)
        .box_from_io_with_msg("Apply batch failed.")?;
    index_db.flush()?;

    Ok(())
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

    #[test]
    fn test_is_whiteout_inode_non() {
        // Create a temporary file
        let temp_file_path = "/home/luxian/megatest/upper/a/hello";
        // Check if the file is a character device
        assert!(is_whiteout_inode(temp_file_path));
    }

    #[test]
    fn test_is_whiteout_inode_invalid_path() {
        // Test with an invalid path
        let invalid_path = "/invalid/path/to/file";
        assert!(!is_whiteout_inode(invalid_path));
    }
}
