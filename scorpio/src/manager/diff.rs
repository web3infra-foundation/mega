use std::collections::HashMap;
use std::{ffi::CString, fs};
use std::path::{Path, PathBuf};
use libc::{self, stat};
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::tree::{Tree, TreeItem, TreeItemMode};
use mercury::internal::object::types::ObjectType;

use crate::manager::store::TreeStore;
use crate::util::GPath;

fn collect_paths<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
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
#[allow(unused)]
fn diff(lower: PathBuf, upper:PathBuf,dbpath:&str)->(HashMap<PathBuf,Tree>,Vec<Blob>){
    let db = sled::open(dbpath).unwrap();
    let upper_changes = collect_paths(&lower); 
    let _root_len = lower.clone();
    let mut map = HashMap::<PathBuf,Tree>::new();
    let root_tree = db.get_bypath(GPath::from(String::from(lower.to_str().unwrap()))).unwrap();
    map.insert(lower.clone(), root_tree);
    let mut blobs=Vec::<Blob>::new();
    for node in upper_changes {
            let p = node.parent().unwrap().to_path_buf();
            let t = map.get_mut(&p).unwrap();

            let mut  is_new_file = true;
            // look up the older version to delete/change . 
            let mut i =0;
            while i<t.tree_items.len() {
                let item = &mut t.tree_items[i];
                if item.name.eq(node.file_name().unwrap().to_string_lossy().as_ref()){
                    is_new_file = false;
                    //delete . 
                    if is_whiteout_inode(&node){
                        t.tree_items.remove(i);
                    }else{
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
#[cfg(test)]
mod tests {
    use fs::File;

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
