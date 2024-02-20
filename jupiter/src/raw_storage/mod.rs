use std::{
    env,
    fs::File,
    io::Read,
    path::{self, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use bytes::Bytes;
use handlebars::Handlebars;

use common::errors::MegaError;
use db_entity::db_enums::StorageType;
use venus::internal::pack::entry::Entry;

use crate::raw_storage::local_storage::LocalStorage;

pub mod local_storage;

#[async_trait]
pub trait RawStorage: Sync + Send {
    fn get_storage_type(&self) -> StorageType;

    async fn get(&self, object_id: &str) -> Result<Bytes, MegaError>;

    async fn put(
        &self,
        object_id: &str,
        size: i64,
        body_content: &[u8],
    ) -> Result<String, MegaError>;

    // save a entry and return the b_link file
    async fn put_entry(&self, entry: &Entry) -> Result<Vec<u8>, MegaError> {
        let location = self
            .put(
                &entry.hash.unwrap().to_plain_str(),
                entry.data.len() as i64,
                &entry.data,
            )
            .await
            .unwrap();
        let handlebars = Handlebars::new();

        let path = env::current_dir().unwrap().join("b_link.txt");
        let mut file = File::open(path).unwrap();
        let mut template = String::new();
        file.read_to_string(&mut template).unwrap();

        let mut context = serde_json::Map::new();
        context.insert(
            "objectType".to_string(),
            serde_json::json!(entry.header.to_string()),
        );
        context.insert(
            "sha1".to_string(),
            serde_json::json!(entry.hash.unwrap().to_plain_str()),
        );
        context.insert(
            "type".to_string(),
            serde_json::json!(self.get_storage_type().to_string()),
        );
        context.insert("location".to_string(), serde_json::json!(location));

        let rendered = handlebars.render_template(&template, &context).unwrap();

        Ok(rendered.into_bytes())
    }

    fn exist(&self, object_id: &str) -> bool;

    async fn list(&self) {
        unreachable!("not implement")
    }

    async fn delete(&self) {
        unreachable!("not implement")
    }

    fn transform_path(&self, path: &str) -> String {
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

pub async fn init(path: String) -> Arc<dyn RawStorage> {
    let storage_type = env::var("MEGA_OBJ_STORAGR_TYPE").unwrap();
    match storage_type.as_str() {
        "LOCAL" => {
            let mut base_path = PathBuf::from(env::var("MEGA_OBJ_LOCAL_PATH").unwrap());
            base_path.push(path);
            Arc::new(LocalStorage::init(base_path))
        }
        // "REMOTE" => Arc::new(RemoteStorage::init(path).await),
        _ => unreachable!(
            "Not supported config, MEGA_OBJ_STORAGR_TYPE should be 'LOCAL' or 'REMOTE'"
        ),
    }
}
