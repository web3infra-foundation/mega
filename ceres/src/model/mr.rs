use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use git_internal::hash::SHA1;

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum MrDiffFile {
    New(PathBuf, SHA1),
    Deleted(PathBuf, SHA1),
    // path, old_hash, new_hash
    Modified(PathBuf, SHA1, SHA1),
}

impl MrDiffFile {
    pub fn path(&self) -> &PathBuf {
        match self {
            MrDiffFile::New(path, _) => path,
            MrDiffFile::Deleted(path, _) => path,
            MrDiffFile::Modified(path, _, _) => path,
        }
    }

    pub fn kind_weight(&self) -> u8 {
        match self {
            MrDiffFile::New(_, _) => 0,
            MrDiffFile::Deleted(_, _) => 1,
            MrDiffFile::Modified(_, _, _) => 2,
        }
    }
}

#[derive(Serialize)]
pub struct BuckFile {
    pub buck: SHA1,
    pub buck_config: SHA1,
    pub path: PathBuf,
}
