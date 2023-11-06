use std::{
    fs::File,
    path::{self},
};

use common::errors::MegaError;

pub mod local_storage;
pub mod remote_storage;

pub trait FileStorage {
    fn get(&self, object_id: &str) -> File;

    fn put(&self, object_id: &str, size: i64, body_content: &[u8]) -> Result<String, MegaError>;

    fn exist(&self, object_id: &str) -> bool;

    fn list(&self) {
        unreachable!("not implement")
    }

    fn delete(&self) {
        unreachable!("not implement")
    }

    fn transform_path(path: &str) -> String {
        if path.len() < 5 {
            path.to_string()
        } else {
            path::Path::new(&path[0..2])
                .join(&path[2..4])
                .join(&path[4..path.len()])
                .into_os_string()
                .into_string()
                .unwrap()
        }
    }
}
