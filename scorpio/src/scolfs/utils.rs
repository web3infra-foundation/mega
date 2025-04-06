use std::{fs, path::PathBuf};
use crate::util::scorpio_config;
// Get the `lfs_path` from the TOML file and return a PathBuf.
pub fn lfs_path() -> PathBuf{
    let store_path_buf = scorpio_config::store_path();
    let mut lfs_path = PathBuf::from(store_path_buf);
    lfs_path.push("scorpio_lfs");
    if !lfs_path.exists() {
        fs::create_dir_all(&lfs_path).expect("Failed to create directory");
    }
    lfs_path

}

// Get the `lfs_attribate` from the TOML file and return a PathBuf.
pub fn lfs_attribate() -> PathBuf{
    let mut lfs_path = lfs_path();
    lfs_path.push(".libra_attribute");
    lfs_path
}

// ==== Helper Functions ====
pub fn current_refspec() -> Option<String> {
    Some("refs/heads/main".to_string())
}
