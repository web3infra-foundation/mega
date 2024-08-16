use std::path::PathBuf;
use crate::utils::util;

pub fn index() -> PathBuf {
    util::storage_path().join("index")
}

pub fn objects() -> PathBuf {
    util::storage_path().join("objects")
}

pub fn database() -> PathBuf {
    util::storage_path().join(util::DATABASE)
}

pub fn attributes() -> PathBuf {
    util::working_dir().join(util::ATTRIBUTES)
}