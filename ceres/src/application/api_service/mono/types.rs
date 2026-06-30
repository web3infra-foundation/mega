use std::{collections::HashMap, path::PathBuf};

use git_internal::{
    hash::ObjectHash,
    internal::object::{blob::Blob, tree::Tree},
};

pub struct TreeUpdateResult {
    pub updated_trees: Vec<Tree>,
    pub ref_updates: Vec<RefUpdate>,
}

pub struct RefUpdate {
    pub path: String,
    pub tree_id: ObjectHash,
}

pub(crate) struct CreateEntryUpdate {
    pub update_result: TreeUpdateResult,
    pub blob: Blob,
    pub entry_oid: ObjectHash,
    pub repo_path: PathBuf,
    pub save_trees: Vec<Tree>,
}

pub(crate) struct ApplyChangeContext<'a> {
    pub components: &'a [String],
    pub chain_paths: &'a [PathBuf],
    pub chain_trees: &'a [Tree],
    pub tree_cache: &'a mut HashMap<PathBuf, Tree>,
    pub new_trees: &'a mut HashMap<ObjectHash, Tree>,
}
