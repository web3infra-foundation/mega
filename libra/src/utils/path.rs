use std::path::PathBuf;
use crate::utils::util;

pub fn index() -> PathBuf {
    util::storage_path().join("index")
}
pub fn objects() -> PathBuf {
    util::storage_path().join("objects")
}