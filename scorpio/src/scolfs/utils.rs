use std::{fs, path::PathBuf};
use super::super::util::config;
// Get the `lfs_path` from the TOML file and return a PathBuf.
pub fn lfs_path() -> PathBuf{
    let mut store_path_buf = config::store_path();
    store_path_buf.push("scorpio_lfs");
    if !store_path_buf.exists() {
        fs::create_dir_all(&store_path_buf).expect("Failed to create directory");
    }
    store_path_buf

}

// Get the `lfs_attribate` from the TOML file and return a PathBuf.
pub fn lfs_attribate() -> PathBuf{
    let mut lfs_path = lfs_path();
    lfs_path.push(".libra_attribute");
}