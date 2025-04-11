use mercury::hash::SHA1;
use mercury::internal::object::types::ObjectType;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::manager::diff::is_whiteout_inode;
use crate::manager::store::{BlobFsStore, ModifiedStore};

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
    real_path: &PathBuf,
    work_path: &Path,
    index_db: &sled::Db,
    rm_db: &sled::Db,
) -> Result<(), Box<dyn std::error::Error>> {
    // Using batch processing to simplify I/O operations
    // and reduce disk consumption.
    let mut index_batch = sled::Batch::default();
    let mut rm_batch = sled::Batch::default();

    println!("\x1b[34m[PART1]\x1b[0m");
    let modified_path = work_path.join("modifiedstore");
    let upper_path = work_path.join("upper");
    let relative_root_path = real_path.strip_prefix(&upper_path)?;

    // HashMap is used here to centrally read Db content to
    // avoid performance loss caused by disk I/O operations.
    let stored_db = index_db.db_list()?;
    let mut stored_path = index_db
        .path_list()?
        .into_iter()
        .filter(|tmp_path| tmp_path.starts_with(relative_root_path))
        .collect::<Vec<PathBuf>>();

    println!("\x1b[34m[PART2]\x1b[0m");
    for entry in WalkDir::new(real_path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() || e.file_type().is_char_device())
    {
        let entry_path = entry.path();
        let path = entry_path.strip_prefix(&upper_path)?.to_path_buf();
        let key = path.to_string_lossy();

        println!("entry.path() = {}", entry_path.display());
        println!("path = {key}");

        if is_whiteout_inode(entry_path) {
            println!("    [\x1b[34mINFO\x1b[0m] whiteout_inode: {key}");
            rm_batch.insert(key.as_bytes(), b"");
            continue;
        }
        let content = std::fs::read(entry_path)?;
        let hash = SHA1::from_type_and_data(ObjectType::Blob, &content);
        match stored_db.get(&path) {
            Some(old_hash) => {
                if !old_hash.eq(&hash._to_string()) {
                    println!("    [\x1b[34mINFO\x1b[0m] Update: {key}");
                    index_batch.insert(key.as_bytes(), hash._to_string().as_bytes());
                    modified_path.add_blob_to_hash(&hash._to_string(), &content)?;
                }
                let index = stored_path.iter().position(|tmp| tmp == &path).unwrap();
                stored_path.remove(index);
            }
            None => {
                println!("    [\x1b[34mINFO\x1b[0m] Add: {key}");
                index_batch.insert(key.as_bytes(), hash._to_string().as_bytes());
                modified_path.add_blob_to_hash(&hash._to_string(), &content)?;
            }
        }
    }

    println!("\x1b[34m[PART3]\x1b[0m");
    let stored_path = stored_path;
    stored_path.iter().for_each(|path| {
        let key = path.to_string_lossy();
        println!("    [\x1b[34mINFO\x1b[0m] Remove: {key}");
        index_batch.remove(key.as_bytes())
    });

    println!("\x1b[34m[PART4]\x1b[0m");
    index_db.apply_batch(index_batch)?;
    rm_db.apply_batch(rm_batch)?;

    index_db.flush()?;
    rm_db.flush()?;

    Ok(())
}
