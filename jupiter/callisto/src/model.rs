

use crate::{mega_blob, mega_snapshot, mega_tree};
use common::utils::generate_id;


impl From<mega_tree::Model> for mega_snapshot::Model {
    fn from(value: mega_tree::Model) -> Self {
        Self {
            id: generate_id(),
            full_path: value.full_path,
            name: value.name,
            sha1: value.tree_id,
            commit_id: value.commit_id,
            size: value.size,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
} 


impl From<mega_blob::Model> for mega_snapshot::Model {
    fn from(value: mega_blob::Model) -> Self {
        Self {
            id: generate_id(),
            full_path: value.full_path,
            name: value.name,
            sha1: value.blob_id,
            commit_id: value.commit_id,
            size: value.size,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
} 