use common::utils::generate_id;
use db_entity::{db_enums::StorageType, raw_objects};

use crate::internal::pack::entry::Entry;

impl From<Entry> for raw_objects::Model {
    fn from(value: Entry) -> Self {
        raw_objects::Model {
            id: generate_id(),
            sha1: value.hash.unwrap().to_plain_str(),
            object_type: String::from_utf8_lossy(value.header.to_bytes()).to_string(),
            storage_type: StorageType::Database,
            data: Some(value.data),
            local_storage_path: None,
            remote_url: None,
        }
    }
}
