use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use mercury::hash::SHA1;

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum ClDiffFile {
    New(PathBuf, SHA1),
    Deleted(PathBuf, SHA1),
    // path, old_hash, new_hash
    Modified(PathBuf, SHA1, SHA1),
}

impl ClDiffFile {
    pub fn path(&self) -> &PathBuf {
        match self {
            ClDiffFile::New(path, _) => path,
            ClDiffFile::Deleted(path, _) => path,
            ClDiffFile::Modified(path, _, _) => path,
        }
    }

    pub fn kind_weight(&self) -> u8 {
        match self {
            ClDiffFile::New(_, _) => 0,
            ClDiffFile::Deleted(_, _) => 1,
            ClDiffFile::Modified(_, _, _) => 2,
        }
    }
}

#[derive(Serialize)]
pub struct BuckFile {
    pub buck: SHA1,
    pub buck_config: SHA1,
    pub path: PathBuf,
}
