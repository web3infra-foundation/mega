use std::collections::HashMap;
use std::vec;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use libc::{self, stat};
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use mercury::internal::object::types::ObjectType;

use crate::manager::store::{StatusStore, TreeStore};

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
fn is_whiteout_inode<P: AsRef<Path>>(path: P)-> bool {
    let c_path = CString::new(path.as_ref().to_string_lossy().as_bytes()).expect("CString::new failed");
    let mut stat_buf: stat = unsafe { std::mem::zeroed() };
    // Call the stat function from libc to get the file status
    let result = unsafe { libc::stat(c_path.as_ptr(), &mut stat_buf) };
    
    // Check if the stat call was successful
    if result == 0 {
        // Check if the file mode is a character device
        return stat_buf.st_mode == (libc::S_IFCHR | 0o777) ;
    }
    
    false
}
/// the output tree in hashmap , which hash value is not computed.
#[allow(unused)]
pub fn diff(lower: PathBuf, upper:PathBuf,dbpath:&str,monopath:PathBuf)->(HashMap<PathBuf,Tree>,Vec<Blob>){
    let db = sled::open(dbpath).unwrap();
    let upper_changes = collect_paths(&lower); 
    let _root_len = lower.clone();
    let mut map = HashMap::<PathBuf,Tree>::new();
    let root_tree = db.get_bypath(monopath.clone()).unwrap();// BUG: wrong path :"lower". the store in db is not real path .
    // dont forget get tree below
    map.insert(lower.clone(), root_tree);
    let mut blobs=Vec::<Blob>::new();

    for node in upper_changes {
        if node.is_dir(){
            let new_path = monopath.clone().join(node.strip_prefix(&lower).unwrap());
            let node_tree = db.get_bypath(new_path).unwrap();
            map.insert(node.clone(), node_tree);
            //db.get_bypath(path);
            //ap.insert(node.clone(), node_tree);
        }
        let p = node.parent().unwrap().to_path_buf();
        let t = map.get_mut(&p).unwrap();

        let mut  is_new_file = true;
        // look up the parent tree node to delete/change . 
        let mut i =0;
        while i<t.tree_items.len() {
            let item = &mut t.tree_items[i];
            if item.name.eq(node.file_name().unwrap().to_string_lossy().as_ref()){
                is_new_file = false;
                //delete . 
                if is_whiteout_inode(&node){
                    t.tree_items.remove(i);
                }
                else if node.is_dir(){
                    //pass
                }
                else{
                    // changed 
                    // node is a changed ()blob.
                    // just compute the NEW hash first.
                    let content = std::fs::read(&node).unwrap();
                    item.id= SHA1::from_type_and_data(ObjectType::Blob, &content);
                    blobs.push(Blob { id: item.id, data: content });
                }
                break;
            }
            i+=1;
        }
        // if new file add item.
        let new_name =node.file_name().unwrap().to_string_lossy().as_ref().to_owned();
        if is_new_file{
            //is a fiel or a dictionary?
            if node.is_dir(){
                // is dictionary. 
                t.tree_items.push(TreeItem {
                    mode: TreeItemMode::Tree, 
                    id: SHA1::default(), 
                    name: new_name,
                });
            }else {
                //is a file.
                let content = std::fs::read(&node).unwrap();
                let hash = SHA1::from_type_and_data(ObjectType::Blob, &content);
                blobs.push(Blob { id: hash, data: content });
                t.tree_items.push(TreeItem {
                    mode: TreeItemMode::Blob, 
                    id: hash, 
                    name: new_name,
                });
            }

        }
    }
    (map,blobs)
}

pub fn change(
    real_path:PathBuf,
    tree_path:PathBuf,
    trees:&mut Vec<Tree>,
    blobs:&mut Vec<Blob>,
    db:&sled::Db) -> Tree{
        let mut tree;
        let root_tree = db.get_bypath(tree_path.clone());
        if let Ok(root_tree) = root_tree{// exit dictionry
            println!("exit tree:{:?}",tree_path);
            tree = root_tree;
        }else {// there is a new dictionary. 
            println!("new tree:{:?}",tree_path);
            tree = Tree{ id: SHA1::default(), tree_items: vec![] };
        }

        let entries = std::fs::read_dir(real_path).unwrap();
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned().to_string();
            let mut new = true;
            let mut i =0 ;
            while i<tree.tree_items.len() {
                let item: &mut TreeItem  = tree.tree_items.get_mut(i).unwrap();
                if item.name.eq(&name) {
                    if is_whiteout_inode(&path){
                        println!("change: delete {}",item.name);
                        // delete..
                        let _ = item;//drop
                        tree.tree_items.remove(i);
                    }
                    else if path.is_dir() {
                        let new_tree_path = tree_path.join(path.file_name().unwrap());
                        let new_tree = change(path.clone(), new_tree_path, trees, blobs, db);

                        // change the hash value & push tree .
                        item.id = new_tree.id;
                        trees.push(new_tree);
                        println!("change: changed tree {}",item.name);
                    } else {
                        println!("change: changed file {}",item.name);
                        let content = std::fs::read(&path).unwrap();
                        let hash = SHA1::from_type_and_data(ObjectType::Blob, &content);
                        blobs.push(Blob { id: hash, data: content });
                        item.id = hash;// change fiel hash .
                    }
                    new = false;
                    break;
                }
                i+=1;
            }
            if new{// a new file or dictionary
                if path.is_dir() {
                    println!("change: new tree  {:?}",name);
                    let new_tree_path = tree_path.join(path.file_name().unwrap());
                    let new_tree = change(path.clone(), new_tree_path, trees, blobs, db);
                    
                    //add a new item for this tree. and push tree.
                    tree.tree_items.push(TreeItem {
                        mode: TreeItemMode::Tree,
                        id: new_tree.id,
                        name ,
                    });
                    trees.push(new_tree);
                } else {
                    println!("change: new file  {}",name);
                    let content = std::fs::read(&path).unwrap();
                    let hash = SHA1::from_type_and_data(ObjectType::Blob, &content);
                    blobs.push(Blob { id: hash, data: content });
                    tree.tree_items.push(TreeItem {
                        mode: TreeItemMode::Blob,
                        id: hash,
                        name ,
                    });
                }
            }


           
        }
        tree.rehash();//Re compute the hash value .
        tree

}

/// This function dosn't check the input path, so if you call it outside the 
/// mono_add() function, be careful the directory injection vulnerability.
///
/// This function should not make any changes to the existing Tree structure,
/// and should only make changes during the Commit operation.
pub fn add_and_del(real_path: PathBuf, tree_path: PathBuf, db: &sled::Db) -> Result<(), Box<dyn std::error::Error>>{
    println!("Start");
    let root_tree = db.get_bypath(tree_path.clone());
    let tree = match root_tree {
        Ok(root_tree) => {
            // exit dictionry
            println!("Exit tree:{:?}", tree_path);
            root_tree
        }
        Err(_) => {
            // there is a new dictionary.
            println!("New tree:{:?}", tree_path);
            Tree {
                id: SHA1::default(),
                tree_items: vec![],
            }
        }
    };

    println!("Processing directory: {}", real_path.display());
    let entries = match std::fs::read_dir(&real_path) {
        Ok(entries) => entries,
        Err(e) => {
            let e_message = format!("Failed to read directory {:?}: {}", real_path, e);
            eprintln!("{e_message}");
            return Err(Box::from(e_message));
        }
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut new = true;

        for item in &tree.tree_items {
            if item.name == name {
                new = false;
                if is_whiteout_inode(&path) {
                    println!("change: Deleting: {}", item.name);
                    match db.delete(path.clone()) {
                        Ok(tmp) => println!("Del: {}", tmp),
                        Err(e) => {
                            let e_message = format!("Failed to delete {:?} from DB: {}", item.name, e);
                            eprintln!("{e_message}");
                            return Err(Box::from(e_message));
                        }
                    }
                } else if path.is_dir() {
                    println!("change: Updating directory: {}", item.name);
                    let new_tree_path = tree_path.join(&name);
                    add_and_del(path.clone(), new_tree_path, db)?;
                } else {
                    // Thanks to the hierarchical structure of OverlayFs, we
                    // can now get all the changed files directly from the
                    // Upper folder without having to read from the mount point
                    // using the FUSE system. This saves a lot of work, such as
                    // Hash verification.
                    /*
                    let content = std::fs::read(&path).unwrap();
                    let new_hash = SHA1::from_type_and_data(ObjectType::Blob, &content);
                    if item.id != new_hash {
                        println!("change: Updating file: {}", item.name);
                        match db.add_content(path.clone(), &content) {
                            Ok(()) => (), // Successfully updated content in DB
                            Err(e) => {
                                let e_message = format!("Failed to update content in DB: {}", e);
                                eprintln!("{e_message}");
                                return Err(Box::from(e_message));
                            },
                        }
                    }
                    */
                    println!("change: Updating file: {}", item.name);
                    match db.add_content(path.clone(), &content) {
                        Ok(()) => (), // Successfully updated content in DB
                        Err(e) => {
                            let e_message = format!("Failed to update content in DB: {}", e);
                            eprintln!("{e_message}");
                            return Err(Box::from(e_message));
                        },
                    }
                }
                break;
            }
        }

        if new {
            if path.is_dir() {
                println!("Adding new directory: {}", name);
                let new_tree_path = tree_path.join(&name);
                add_and_del(path.clone(), new_tree_path, db)?;
            } else {
                println!("Adding new file: {}", name);
                let content = std::fs::read(&path).unwrap();
                match db.add_content(path.clone(), &content) {
                    Ok(()) => (),
                    Err(e) => {
                        let e_message = format!("Failed to update content in DB: {}", e);
                        eprintln!("{e_message}");
                        return Err(Box::from(e_message));
                    },
                }
            }
        }
    }

    match db.insert_tree(tree_path, tree.clone()) {
        Ok(()) => {
            println!("Add operation completed successfully");
            Ok(())
        }
        Err(e) => {
            let e_message = format!("Failed to update content in DB: {}", e);
            eprintln!("{e_message}");
            return Err(Box::from(e_message));
        }
    }
}

#[cfg(test)]
mod tests {

    use std::fs::File;

    use super::*;


    #[test]
    fn test_collect_paths_nested_directories() {
        let temp_dir = "/home/luxian/code/mega/scorpio/src";
        let paths = collect_paths(temp_dir);
        for p in paths{
            println!("{:?}",p);
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
