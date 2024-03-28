use serde::{Deserialize, Serialize};

use callisto::mega_refs;
use common::utils::MEGA_BRANCH_NAME;

use crate::internal::pack::reference::Refs;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct MegaRefs {
    pub id: i64,
    pub path: String,
    pub ref_commit_hash: String,
    pub ref_tree_hash: String,
}

impl From<mega_refs::Model> for MegaRefs {
    fn from(value: mega_refs::Model) -> Self {
        Self {
            id: value.id,
            path: value.path,
            ref_commit_hash: value.ref_commit_hash,
            ref_tree_hash: value.ref_tree_hash,
        }
    }
}

impl From<MegaRefs> for mega_refs::Model {
    fn from(value: MegaRefs) -> Self {
        Self {
            id: value.id,
            path: value.path,
            ref_commit_hash: value.ref_commit_hash,
            ref_tree_hash: value.ref_tree_hash,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<MegaRefs> for Refs {
    fn from(value: MegaRefs) -> Self {
        Self {
            id: value.id,
            ref_hash: value.ref_commit_hash,
            default_branch: true,
            ref_name: MEGA_BRANCH_NAME.to_owned(),
        }
    }
}
