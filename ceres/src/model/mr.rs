use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use mercury::hash::SHA1;

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum MrDiffFile {
    New(PathBuf, SHA1),
    Deleted(PathBuf, SHA1),
    // path, old_hash, new_hash
    Modified(PathBuf, SHA1, SHA1),
}
