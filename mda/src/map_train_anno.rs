//! Used to map the traning data and its annotation data
//! There are two cases:
//! 1. The training data and annotation data are stored in separate folders, 
//! with the training data files having the same filenames as the annotation data files.
//! 
//! 2. All the training data is stored in CSV or JSON files and needs to be parsed and matche(TODO)

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Get files in the folder
pub fn get_files_in_folder(folder_path: &str) -> Vec<PathBuf> {
    fs::read_dir(folder_path)
        .expect("Failed to read folder contents")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.path())
        .collect()
}

/// Map files in different folder
pub fn combine_files(a_files: Vec<PathBuf>, b_files: Vec<PathBuf>) -> HashMap<PathBuf, PathBuf> {
    let mut file_combinations = HashMap::new();

    for a_file in a_files {
        let a_file_stem = a_file.file_stem().unwrap();
        for b_file in &b_files {
            let b_file_stem = b_file.file_stem().unwrap();
            if a_file_stem == b_file_stem {
                file_combinations.insert(a_file.clone(), b_file.clone());
                break;
            }
        }
    }

    file_combinations
}
