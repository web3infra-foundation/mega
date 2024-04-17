#![cfg(test)]

use std::{env, fs, path::PathBuf};

use super::util;

pub const TEST_DIR: &str = "libra_test_repo";
/* tools for test */
fn find_cargo_dir() -> PathBuf {
    let cargo_path = env::var("CARGO_MANIFEST_DIR");
    match cargo_path {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            // vscode DEBUG test没有CARGO_MANIFEST_DIR宏，手动尝试查找cargo.toml
            println!("CARGO_MANIFEST_DIR not found, try to find Cargo.toml manually");
            let mut path = util::cur_dir();
            loop {
                path.push("Cargo.toml");
                if path.exists() {
                    break;
                }
                if !path.pop() {
                    panic!("找不到CARGO_MANIFEST_DIR");
                }
            }
            path.pop();
            path
        }
    }
}

/// switch cur_dir to test_dir
fn setup_env() {
    // color_backtrace::install(); // colorize backtrace

    let mut path = find_cargo_dir();
    path.push(TEST_DIR);
    if !path.exists() {
        fs::create_dir(&path).unwrap();
    }
    env::set_current_dir(&path).unwrap(); // 将执行目录切换到测试目录
}

// pub async fn init_repo() {
//     crate::command::init().await.unwrap();
// }
//
// /// with 初始化的干净的mit
// pub fn setup_with_clean_mit() {
//     setup_without_mit();
//     init_repo();
// }

/// switch to test dir and clean .libra
pub fn setup_without_libra() {
    setup_env();
    let mut path = util::cur_dir();
    path.push(util::ROOT_DIR);
    if path.exists() {
        fs::remove_dir_all(&path).unwrap();
    }
}

// pub fn ensure_files<T: AsRef<str>>(paths: &Vec<T>) {
//     for path in paths {
//         ensure_file(path.as_ref().as_ref(), None);
//     }
// }
//
// pub fn ensure_empty_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
//     let entries = fs::read_dir(path.as_ref())?;
//     for entry in entries {
//         let path = entry?.path();
//         if path.is_dir() {
//             fs::remove_dir_all(&path)?; // 如果是目录，则递归删除
//         } else {
//             fs::remove_file(&path)?; // 如果是文件，则直接删除
//         }
//     }
//     Ok(())
// }
//
// pub fn setup_with_empty_workdir() {
//     let test_dir = find_cargo_dir().join(TEST_DIR);
//     ensure_empty_dir(&test_dir).unwrap();
//     setup_with_clean_mit();
// }
//
// pub fn ensure_file(path: &Path, content: Option<&str>) {
//     // 以测试目录为根目录，创建文件
//     fs::create_dir_all(path.parent().unwrap()).unwrap(); // ensure父目录
//     let mut file = fs::File::create(util::get_working_dir().unwrap().join(path))
//         .expect(format!("无法创建文件：{:?}", path).as_str());
//     if let Some(content) = content {
//         file.write(content.as_bytes()).unwrap();
//     } else {
//         // 写入文件名
//         file.write(path.file_name().unwrap().to_str().unwrap().as_bytes()).unwrap();
//     }
// }
//
// pub fn ensure_no_file(path: &Path) {
//     // 以测试目录为根目录，删除文件
//     if path.exists() {
//         fs::remove_file(util::get_working_dir().unwrap().join(path)).unwrap();
//     }
// }
//
// /** 列出子文件夹 */
// pub fn list_subdir(path: &Path) -> io::Result<Vec<PathBuf>> {
//     let mut files = Vec::new();
//     let path = path.to_absolute();
//     if path.is_dir() {
//         for entry in fs::read_dir(path)? {
//             let entry = entry?;
//             let path = entry.path();
//             if path.is_dir() && path.file_name().unwrap_or_default() != util::ROOT_DIR {
//                 files.push(path)
//             }
//         }
//     }
//     Ok(files)
// }
