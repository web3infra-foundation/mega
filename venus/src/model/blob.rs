use callisto::{db_enums::MergeStatus, mega_blob};
use common::utils::generate_id;

use crate::internal::object::blob::Blob;


impl From<Blob> for mega_blob::Model {
    fn from(value: Blob) -> Self {
        mega_blob::Model {
            id: generate_id(),
            blob_id: value.id.to_plain_str(),
            mr_id: String::new(),
            status: MergeStatus::Merged,
            size: 0,
            full_path: String::new(),
            commit_id: String::new(),
            name: String::new(),
            data: value.data,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}