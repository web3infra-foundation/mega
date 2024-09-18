use std::{ffi::CString, fs};
use std::path::Path;
use libc::{self, stat};
use mercury::hash::SHA1;

fn collect_paths<P: AsRef<Path>>(path: P) -> Vec<String> {
    let mut paths = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                paths.push(path.display().to_string());
                paths.extend(collect_paths(&path));
            } else {
                paths.push(path.display().to_string());
            }
        }
    }

    paths
}

// is a file or file node is whiteout file?
fn is_whiteout_inode(path: &String) -> bool {
    let c_path = CString::new(path.as_bytes()).expect("CString::new failed");
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
fn diff(lower: String, upper:String){
    let upper_changes = collect_paths(&lower); 
    let _root_len = lower.len();
    for node in upper_changes {
            if is_whiteout_inode(&node){
                // delte a bolb from a tree .
            }
            else{
                let path = Path::new(&node);
                if path.is_dir(){
                    // fix a tree struct. 

                }else{
                    // node is a file ()blob.
                    // just compute the HASH first.
                    let content = std::fs::read(&node).unwrap();
                    let haah = SHA1::from_type_and_data(mercury::internal::object::types::ObjectType::Blob, &content);
                }
            }
    }
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
            println!("{}",p);
        }
    }

        #[test]
        fn test_is_whiteout_inode() {
            // Create a temporary file
            let temp_file_path = "temp_file.txt";
            let _file = File::create(temp_file_path).expect("Unable to create file");
            
            // Check if the file is a character device
            assert!(!is_whiteout_inode(&temp_file_path.to_string()));
            
            // Clean up the temporary file
            std::fs::remove_file(temp_file_path).expect("Unable to delete file");
        }

        #[test]
        fn test_is_whiteout_inode_non() {
            // Create a temporary file
            let temp_file_path = "/home/luxian/megatest/upper/a/hello";
            // Check if the file is a character device
            assert!(is_whiteout_inode(&temp_file_path.to_string()));
            
        }

        #[test]
        fn test_is_whiteout_inode_invalid_path() {
            // Test with an invalid path
            let invalid_path = "/invalid/path/to/file";
            assert!(!is_whiteout_inode(&invalid_path.to_string()));
        }
}
