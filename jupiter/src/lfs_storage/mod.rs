use std::{
    path::{self, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use bytes::Bytes;

use common::errors::MegaError;

use crate::lfs_storage::local_storage::LocalStorage;

pub mod local_storage;

#[derive(Debug, Clone, Default)]
pub struct BlobLink {
    pub version: String,
    pub object_type: String,
    pub storage_type: String,
    pub storge_location: String,
}

#[async_trait]
pub trait LfsStorage: Sync + Send {

    async fn get_ref(&self, repo_id: i64, ref_name: &str) -> Result<String, MegaError>;

    async fn put_ref(
        &self,
        repo_id: i64,
        ref_name: &str,
        ref_hash: &str,
    ) -> Result<(), MegaError>;

    async fn delete_ref(&self, repo_id: i64, ref_name: &str) -> Result<(), MegaError>;

    async fn update_ref(
        &self,
        repo_id: i64,
        ref_name: &str,
        ref_hash: &str,
    ) -> Result<(), MegaError>;

    async fn get_object(&self, object_id: &str) -> Result<Bytes, MegaError>;

    async fn put_object(
        &self,
        object_id: &str,
        body_content: &[u8],
    ) -> Result<String, MegaError>;

    // async fn parse_blob_link(&self, data: Vec<u8>) -> Result<BlobLink, MegaError> {
    //     let mut reader = BufReader::new(data.as_slice());
    //     let mut blink = BlobLink::default();
    //     // for line in reader.lines() {
    //     //     let str = line.unwrap();
    //     // }
    //     let mut buf = String::new();
    //     reader.read_line(&mut buf).unwrap();
    //     blink.version = buf.split_whitespace().next();
    //     let result = self.get_by_path(&blink.storge_location).await.unwrap();
    //     Ok(blink)
    // }

    // save a entry and return the b_link file
    // async fn convert_blink(&self, entry: &Entry) -> Result<Vec<u8>, MegaError> {
    //     let location = self
    //         .put_object( &entry.hash.to_plain_str(), &entry.data)
    //         .await
    //         .unwrap();
    //     let handlebars = Handlebars::new();

    //     let path = env::current_dir().unwrap().join("b_link.txt");
    //     let mut file = File::open(path).unwrap();
    //     let mut template = String::new();
    //     file.read_to_string(&mut template).unwrap();

    //     let mut context = serde_json::Map::new();
    //     context.insert(
    //         "objectType".to_string(),
    //         serde_json::json!(entry.obj_type.to_string()),
    //     );
    //     context.insert(
    //         "sha1".to_string(),
    //         serde_json::json!(entry.hash.to_plain_str()),
    //     );
    //     context.insert(
    //         "type".to_string(),
    //         serde_json::json!(self.get_storage_type().to_string()),
    //     );
    //     context.insert("location".to_string(), serde_json::json!(location));

    //     let rendered = handlebars.render_template(&template, &context).unwrap();

    //     Ok(rendered.into_bytes())
    // }

    fn exist_object(&self, object_id: &str) -> bool;

    fn transform_path(&self, sha1: &str) -> String {
        if sha1.len() < 5 {
            sha1.to_string()
        } else {
            path::Path::new(&sha1[0..2])
                .join(&sha1[2..4])
                .join(&sha1[4..sha1.len()])
                .into_os_string()
                .into_string()
                .unwrap()
        }
    }
}

pub async fn init(storage_type: String, base_path: PathBuf) -> Arc<dyn LfsStorage> {
    match storage_type.as_str() {
        "LOCAL" => {
            Arc::new(LocalStorage::init(base_path))
        }
        // "REMOTE" => Arc::new(RemoteStorage::init(path).await),
        _ => unreachable!(
            "Not supported config, MEGA_OBJ_STORAGE_TYPE should be 'LOCAL' or 'REMOTE'"
        ),
    }
}

pub fn mock() -> Arc<dyn LfsStorage> {
    Arc::new(LocalStorage::init(PathBuf::from("/")))
}
