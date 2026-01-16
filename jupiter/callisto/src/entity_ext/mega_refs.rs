use std::path::Path;

use crate::{entity_ext::generate_id, mega_refs};

impl mega_refs::Model {
    pub fn new<P: AsRef<Path>>(
        path: P,
        ref_name: String,
        ref_commit_hash: String,
        ref_tree_hash: String,
        is_cl: bool,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: generate_id(),
            path: path.as_ref().to_str().unwrap().to_string(),
            ref_name,
            ref_commit_hash,
            ref_tree_hash,
            created_at: now,
            updated_at: now,
            is_cl,
        }
    }
}
