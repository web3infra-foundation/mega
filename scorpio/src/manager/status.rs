use std::collections::HashSet;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::manager::diff::is_whiteout_inode;
use crate::manager::store::ModifiedStore;

fn get_difference(hashset_a: &HashSet<PathBuf>, hashset_b: &HashSet<PathBuf>) -> HashSet<PathBuf> {
    hashset_a
        .difference(&hashset_b)
        .cloned()
        .collect::<HashSet<PathBuf>>()
}

fn get_intersection(hashset_a: &HashSet<PathBuf>, hashset_b: &HashSet<PathBuf>) -> HashSet<PathBuf> {
    hashset_a
        .intersection(&hashset_b)
        .cloned()
        .collect::<HashSet<PathBuf>>()
}



fn walk_paths(root: &PathBuf) -> HashSet<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() || e.file_type().is_char_device())
        .map(|e| e.path().strip_prefix(root).unwrap().to_path_buf())
        .collect()
}

/// The core function of status operation.
///
/// This function dosn't check the input path, so if you call it outside the
/// mono_add() function, be careful the directory injection vulnerability.
pub fn status_core(
    work_path: &Path,
    index_db: &sled::Db,
    rm_db: &sled::Db,
) -> Result<Box<String>, Box<dyn std::error::Error>> {
    let added_files = index_db
        .path_list()?
        .into_iter()
        .collect::<HashSet<PathBuf>>();
    let added_removed_files = rm_db.path_list()?.into_iter().collect::<HashSet<PathBuf>>();

    let lower_path = work_path.join("lower");
    let upper_path = work_path.join("upper");

    let lower_content = walk_paths(&lower_path);
    let upper_content = walk_paths(&upper_path);

    // Files that exist in Upper but not in Lower are newly
    // created files.
    let new_files = get_difference(&upper_content, &lower_content);

    // Newly created files that are not recorded in the temporary
    // storage area are untracked files.
    let untracked_files = get_difference(&new_files, &added_files);

    // Since the difference between modification and deletion in
    // Upper cannot be directly reflected, we still use the
    // is_char_device or is_white_out function.
    let same_part = get_intersection(&upper_content, &lower_content);
    let tmp = get_difference(&same_part, &added_files);
    let unstored_part = get_difference(&tmp, &added_removed_files);
    let unstored_del = unstored_part
        .iter()
        .filter(|tmp| is_whiteout_inode(upper_path.join(tmp)))
        .cloned()
        .collect::<HashSet<PathBuf>>();

    let mut buffer = String::with_capacity(2000);
    
    if !(added_files.is_empty() && added_removed_files.is_empty()) {
        buffer += "Changes to be committed:\n";
        buffer += "  (use \"git restore --staged <file>...\" to unstage)\n\x1b[32m";
        for tmp in new_files.difference(&untracked_files) {
            buffer += format!("\tnew file:\t{}\n", tmp.display()).as_str();
        }
        for tmp in added_removed_files {
            buffer += format!("\tdeleted:\t{}\n", tmp.display()).as_str();
        }
        // All files in the temporary storage area except newly created
        // files are modified files.
        for tmp in added_files.difference(&new_files) {
            buffer += format!("\tmodified:\t{}\n", tmp.display()).as_str();
        }

        buffer += "\x1b[0m\n";
    }

    if !unstored_part.is_empty() {
        buffer += "Changes not staged for commit:\n";
        buffer += "  (use \"git add/rm <file>...\" to update what will be committed)\n";
        buffer +=
            "  (use \"git restore <file>....\" to discard changes in working directory)\n\x1b[31m";
        for tmp in unstored_del.iter() {
            buffer += format!("\tdeleted:\t{}\n", tmp.display()).as_str();
        }
        for tmp in unstored_part.difference(&unstored_del) {
            buffer += format!("\tmodified:\t{}\n", tmp.display()).as_str();
        }

        buffer += "\x1b[0m\n";
    }

    if !untracked_files.is_empty() {
        buffer += "Untracked files:\n";
        buffer += "  (use \"git add <file>...\" to include in what will be committed)\n\x1b[31m";
        for tmp in untracked_files {
            buffer += format!("\t{}", tmp.display()).as_str();
        }

        buffer += "\x1b[0m";
    }

    let buffer = Box::new(buffer);
    println!("{buffer}");

    Ok(buffer)
}
