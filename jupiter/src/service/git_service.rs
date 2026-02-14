use bytes::Bytes;
use common::errors::MegaError;
use common::utils::is_full_hex_object_id;
use futures::StreamExt;
use git_internal::internal::object::blob::Blob;
use io_orbit::{
    factory::MegaObjectStorageWrapper,
    object_storage::{
        MultiObjectByteStream, ObjectByteStream, ObjectKey, ObjectMeta, ObjectNamespace,
    },
};

use crate::utils::into_obj_stream::IntoObjectStream;

#[derive(Clone)]
pub struct GitService {
    pub obj_storage: MegaObjectStorageWrapper,
}

impl GitService {
    pub fn mock() -> Self {
        Self {
            obj_storage: MegaObjectStorageWrapper::mock(),
        }
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
            .inner
            .put_stream(&key, Box::pin(blob.into_stream()), meta)
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
            .inner
            .put_stream(&key, Box::pin(raw_data.into_stream()), meta)
            .await;

        let _: () = if let Err(e) = res {
            tracing::debug!("Failed to upload blob {:?}: {:?}", key, e);
            return Err(e);
        };
        Ok(())
    }

    pub async fn get_object_as_bytes(&self, hash: &str) -> Result<Vec<u8>, MegaError> {
        // Avoid sending obviously invalid object ids to object storage backends.
        if !is_full_hex_object_id(hash) {
            return Err(MegaError::Other("Invalid object ID format".to_string()));
        }

        let key = ObjectKey {
            namespace: ObjectNamespace::Git,
            key: hash.to_string(),
        };

        let (mut stream, _meta) = self.obj_storage.inner.get_stream(&key).await?;

        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            data.extend_from_slice(&chunk);
        }

        Ok(data)
    }

    pub fn get_objects_stream(&self, hashes: Vec<String>) -> MultiObjectByteStream<'_> {
        // Filter out obviously invalid object ids early to avoid spurious backend requests.
        // Callers that need strict validation should validate up-front and return 4xx.
        let hashes = hashes
            .into_iter()
            .filter(|h| is_full_hex_object_id(h))
            .collect::<Vec<_>>();
        self.obj_storage.inner.get_many(
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
        self.obj_storage.inner.put_many(objects, 16).await
    }
}
