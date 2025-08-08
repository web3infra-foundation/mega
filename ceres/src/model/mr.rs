use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use utoipa::ToSchema;
use mercury::hash::SHA1;

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

    pub fn kind_weight(&self) -> &PathBuf {
        match self {
            MrDiffFile::New(path, _) 
            | MrDiffFile::Deleted(path, _)
            | MrDiffFile::Modified(path, _, _) => path,
        }
    }
}

#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct MrDiff {
    pub data: String,
    pub page_info: Option<MrPageInfo>,
}

#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct MrPageInfo {
    pub total_pages: usize,
    pub current_page: usize,
    pub page_size: usize,
}

#[derive(Serialize)]
pub struct BuckFile {
    pub buck: SHA1,
    pub buck_config: SHA1,
    pub path: PathBuf,
}
