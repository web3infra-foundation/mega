use std::str::FromStr;

use callisto::{db_enums::StorageType, git_blob, mega_blob, raw_blob};
use common::utils::generate_id;

use crate::{hash::SHA1, internal::object::blob::Blob};

impl From<Blob> for mega_blob::Model {
    fn from(value: Blob) -> Self {
        mega_blob::Model {
            id: generate_id(),
            blob_id: value.id.to_plain_str(),
            size: 0,
            commit_id: String::new(),
            name: String::new(),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<Blob> for git_blob::Model {
    fn from(value: Blob) -> Self {
        git_blob::Model {
            id: generate_id(),
            repo_id: 0,
            blob_id: value.id.to_plain_str(),
            size: 0,
            commit_id: String::new(),
            name: String::new(),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<Blob> for raw_blob::Model {
    fn from(value: Blob) -> Self {
        raw_blob::Model {
            id: generate_id(),
            sha1: value.id.to_plain_str(),
            storage_type: StorageType::Database,
            data: Some(value.data),
            content: None,
            file_type: None,
            local_path: None,
            remote_url: None,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<raw_blob::Model> for Blob {
    fn from(value: raw_blob::Model) -> Self {
        Blob {
            id: SHA1::from_str(&value.sha1).unwrap(),
            data: value.data.unwrap(),
        }
    }
}
