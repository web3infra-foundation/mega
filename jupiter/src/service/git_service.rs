use std::sync::Arc;

use bytes::Bytes;
use common::errors::MegaError;
use futures::StreamExt;
use git_internal::internal::object::blob::Blob;

use crate::object_storage::{
    MultiObjectByteStream, ObjectByteStream, ObjectKey, ObjectMeta, ObjectNamespace, ObjectStorage,
    fs_object_storage::FsObjectStorage, object_stream::IntoObjectStream,
};

#[derive(Clone)]
pub struct GitService {
    pub obj_storage: Arc<dyn ObjectStorage>,
}

impl GitService {
    pub fn mock() -> Self {
        let obj_storage = Arc::new(FsObjectStorage::new("/tmp/mega_test_object_storage"));

        Self { obj_storage }
    }

    pub async fn save_object_from_raw(&self, bytes: Bytes) -> Result<String, MegaError> {
        let blob = Blob::from_content_bytes(bytes.to_vec());

        let blob_id = blob.id.clone().to_string();

        let key = ObjectKey {
            namespace: ObjectNamespace::Git,
            key: blob.id.to_string(),
        };

        let meta = ObjectMeta {
            size: blob.data.len() as i64,
            content_type: Some("application/octet-stream".to_string()),
            ..Default::default()
        };

        let res = self
            .obj_storage
            .put(&key, Box::pin(blob.into_stream()), meta)
            .await;

        Ok(if let Err(e) = res {
            tracing::debug!("Failed to upload blob {:?}: {:?}", key, e);
            return Err(e);
        } else {
            blob_id
        })
    }

    pub async fn save_object_from_model(
        &self,
        raw_data: Vec<u8>,
        id: &str,
    ) -> Result<(), MegaError> {
        let key = ObjectKey {
            namespace: ObjectNamespace::Git,
            key: id.to_string(),
        };

        let meta = ObjectMeta {
            size: raw_data.len() as i64,
            content_type: Some("application/octet-stream".to_string()),
            ..Default::default()
        };

        let res = self
            .obj_storage
            .put(&key, Box::pin(raw_data.into_stream()), meta)
            .await;

        let _: () = if let Err(e) = res {
            tracing::debug!("Failed to upload blob {:?}: {:?}", key, e);
            return Err(e);
        };
        Ok(())
    }

    pub async fn get_object_as_bytes(&self, hash: &str) -> Result<Vec<u8>, MegaError> {
        let key = ObjectKey {
            namespace: ObjectNamespace::Git,
            key: hash.to_string(),
        };

        let (mut stream, _meta) = self.obj_storage.get(&key).await?;

        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            data.extend_from_slice(&chunk);
        }

        Ok(data)
    }

    pub fn get_objects_stream(&self, hashes: Vec<String>) -> MultiObjectByteStream<'_> {
        self.obj_storage.get_many(
            hashes
                .into_iter()
                .map(|hash| ObjectKey {
                    namespace: ObjectNamespace::Git,
                    key: hash,
                })
                .collect(),
            16,
        )
    }

    pub async fn put_objects(&self, objects: Vec<Blob>) -> Result<(), MegaError> {
        if objects.len() >= 1000 {
            return Err(MegaError::Other(format!(
                "put_objects called with {} objects; large batches should use put_objects_stream directly",
                objects.len()
            )));
        }
        let raw_blobs: MultiObjectByteStream<'_> =
            Box::pin(futures::stream::iter(objects.into_iter().map(|blob| {
                let key = ObjectKey {
                    namespace: ObjectNamespace::Git,
                    key: blob.id.to_string(),
                };
                let meta = ObjectMeta {
                    size: blob.data.len() as i64,
                    ..Default::default()
                };
                let stream: ObjectByteStream = blob.into_stream();

                Ok((key, stream, meta))
            })));
        self.put_objects_stream(raw_blobs).await
    }

    pub async fn put_objects_stream(
        &self,
        objects: MultiObjectByteStream<'_>,
    ) -> Result<(), MegaError> {
        self.obj_storage.put_many(objects, 16).await
    }
}
