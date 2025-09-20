use std::str::FromStr;

use crate::internal::model::sea_models::{git_blob as sea_git_blob, mega_blob as sea_mega_blob, raw_blob as sea_raw_blob};
use crate::internal::model::generate_id;
use crate::{hash::SHA1, internal::object::blob::Blob};

impl From<&Blob> for sea_mega_blob::Model {
    fn from(value: &Blob) -> Self {
        sea_mega_blob::Model {
            id: generate_id(),
            blob_id: value.id.to_string(),
            size: 0,
            commit_id: String::new(),
            name: String::new(),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<&Blob> for sea_git_blob::Model {
    fn from(value: &Blob) -> Self {
        sea_git_blob::Model {
            id: generate_id(),
            repo_id: 0,
            blob_id: value.id.to_string(),
            size: 0,
            name: None,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<&Blob> for sea_raw_blob::Model {
    fn from(value: &Blob) -> Self {
        sea_raw_blob::Model {
            id: generate_id(),
            sha1: value.id.to_string(),
            storage_type: "database".to_string(),
            data: Some(value.data.clone()),
            content: None,
            file_type: None,
            local_path: None,
            remote_url: None,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<sea_raw_blob::Model> for Blob {
    fn from(value: sea_raw_blob::Model) -> Self {
        Blob {
            id: SHA1::from_str(&value.sha1).unwrap(),
            data: value.data.unwrap(),
        }
    }
}
