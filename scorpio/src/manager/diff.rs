use std::{ffi::CString, fs};
use std::path::Path;
use libc::{self, stat, S_IFCHR};
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
    // 调用 libc 的 stat 函数获取文件状态
    let result = unsafe { libc::stat(c_path.as_ptr(), &mut stat_buf) };
    
    // 检查 stat 调用是否成功
    if result == 0 {
        // 检查文件模式是否为字符设备
        return stat_buf.st_mode == (libc::S_IFCHR | 0o777) ;
    }
    
    false
}

fn diff(lower: String, upper:String){
    let upper_changes = collect_paths(&lower); 
    let root_len = lower.len();
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
        // 创建一个临时文件
        let temp_file_path = "temp_file.txt";
        let _file = File::create(temp_file_path).expect("Unable to create file");
        
        // 检查该文件是否为字符设备
        assert!(!is_whiteout_inode(&temp_file_path.to_string()));
        
        // 清理临时文件
        std::fs::remove_file(temp_file_path).expect("Unable to delete file");
    }


    #[test]
    fn test_is_whiteout_inode_non() {
        // 创建一个临时文件
        let temp_file_path = "/home/luxian/megatest/upper/a/hello";
        // 检查该文件是否为字符设备
        assert!(is_whiteout_inode(&temp_file_path.to_string()));
        
    }


    #[test]
    fn test_is_whiteout_inode_invalid_path() {
        // 测试无效路径
        let invalid_path = "/invalid/path/to/file";
        assert!(!is_whiteout_inode(&invalid_path.to_string()));
    }
}
