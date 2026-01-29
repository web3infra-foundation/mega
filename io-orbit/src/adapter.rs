use std::{sync::Arc, time::Duration};

use bytes::{Bytes, BytesMut};
use common::errors::MegaError;
use futures::{TryStreamExt, stream};
use object_store::{
    ObjectStore, ObjectStoreExt, PutPayload, aws::AmazonS3, gcp::GoogleCloudStorage,
    local::LocalFileSystem, signer::Signer,
};
use reqwest::Method;

use crate::{
    error::IoOrbitError,
    object_storage::{MegaObjectStorage, ObjectByteStream, ObjectKey, ObjectMeta},
};

/// Strategy used for uploading objects to the underlying [`BackendStore`].
///
/// This controls whether data is sent in a single request (`SinglePut`) or
/// split into multiple parts (`Multipart`) when supported by the backend.
/// Callers select a strategy based on object size, latency requirements,
/// and backend capabilities.
pub enum UploadStrategy {
    /// Upload the object using a multipart/streaming upload when supported.
    Multipart,
    /// Upload the entire object using a single `PUT`-style request.
    SinglePut,
}

/// Adapter that exposes an [`ObjectStore`] backend through the
/// [`MegaObjectStorage`] trait.
///
/// This type holds a concrete [`BackendStore`] implementation and an
/// [`UploadStrategy`] that determines how uploads are performed. It is the
/// main integration point between Mega's storage abstraction and the
/// `object_store` crate backends such as S3, GCS, or the local filesystem.
pub struct ObjectStoreAdapter {
    /// The concrete backend store used for all object operations.
    pub store: BackendStore,
    /// The upload strategy used when writing new objects.
    pub upload_strategy: UploadStrategy,
}

/// Supported backend implementations for object storage.
///
/// Each variant wraps a specific `object_store` backend in an [`Arc`] so that
/// a single instance can be cheaply shared across multiple adapters or tasks.
/// New backends should be added as additional enum variants.
pub enum BackendStore {
    /// Amazon S3-compatible object storage backend.
    S3(Arc<AmazonS3>),
    /// Google Cloud Storage backend.
    Gcs(Arc<GoogleCloudStorage>),
    /// Local filesystem backend, primarily for development and testing.
    Local(Arc<LocalFileSystem>),
}

#[async_trait::async_trait]
impl MegaObjectStorage for ObjectStoreAdapter {
    async fn put_stream(
        &self,
        key: &ObjectKey,
        data: ObjectByteStream,
        _meta: ObjectMeta,
    ) -> Result<(), MegaError> {
        let path = key.to_object_store_path();

        match self.upload_strategy {
            UploadStrategy::Multipart => self.put_multipart(&path, data).await,
            UploadStrategy::SinglePut => self.put_single(&path, data).await,
        }
    }

    async fn get_stream(
        &self,
        key: &ObjectKey,
    ) -> Result<(ObjectByteStream, ObjectMeta), MegaError> {
        let path = key.to_object_store_path();

        let stream = self
            .to_store()
            .get(&path)
            .await
            .map_err(IoOrbitError::from)?
            .into_stream();

        Ok((
            Box::pin(stream.map_err(std::io::Error::other)),
            ObjectMeta::default(),
        ))
    }

    async fn get_range_stream(
        &self,
        key: &ObjectKey,
        start: u64,
        end: Option<u64>,
    ) -> Result<(ObjectByteStream, ObjectMeta), MegaError> {
        let path = key.to_object_store_path();

        // Use object_store's Range support
        // object_store 0.13+ supports Range via GetRange

        // object_store 0.13's `get_range` takes `Range<u64>`.
        // If `end` is not provided, we resolve it via `head()` to get object size.
        let end = match end {
            Some(end) => end,
            None => {
                self.to_store()
                    .head(&path)
                    .await
                    .map_err(IoOrbitError::from)?
                    .size
            }
        };

        let bytes = self
            .to_store()
            .get_range(&path, start..end)
            .await
            .map_err(IoOrbitError::from)?;

        // `get_range` returns fully-buffered Bytes, adapt to our streaming type.
        let stream = stream::once(async move { Ok::<Bytes, std::io::Error>(bytes) });

        Ok((Box::pin(stream), ObjectMeta::default()))
    }

    async fn signed_url(
        &self,
        key: &ObjectKey,
        method: Method,
        expires_in: Duration,
    ) -> Result<Option<String>, MegaError> {
        let path = key.to_object_store_path();

        let url = match &self.store {
            BackendStore::S3(s3) => Some(
                s3.signed_url(method, &path, expires_in)
                    .await
                    .map_err(IoOrbitError::from)?
                    .to_string(),
            ),
            BackendStore::Gcs(gcs) => Some(
                gcs.signed_url(method, &path, expires_in)
                    .await
                    .map_err(IoOrbitError::from)?
                    .to_string(),
            ),
            BackendStore::Local(_) => None,
        };
        Ok(url)
    }

    async fn exists(&self, key: &ObjectKey) -> Result<bool, MegaError> {
        let path = key.to_object_store_path();
        Ok(self.to_store().head(&path).await.is_ok())
    }

    async fn delete(&self, key: &ObjectKey) -> Result<(), MegaError> {
        let path = key.to_object_store_path();
        self.to_store()
            .delete(&path)
            .await
            .map_err(IoOrbitError::from)?;
        Ok(())
    }
}

impl ObjectStoreAdapter {
    fn to_store(&self) -> &dyn ObjectStore {
        let store: &dyn ObjectStore = match &self.store {
            BackendStore::S3(s3) => s3.as_ref(),
            BackendStore::Gcs(gcs) => gcs.as_ref(),
            BackendStore::Local(local) => local.as_ref(),
        };
        store
    }

    async fn put_multipart(
        &self,
        path: &object_store::path::Path,
        mut data: ObjectByteStream,
    ) -> Result<(), MegaError> {
        let mut upload = self
            .to_store()
            .put_multipart(path)
            .await
            .map_err(IoOrbitError::from)?;

        let res = async {
            while let Some(chunk) = data.try_next().await? {
                upload
                    .put_part(chunk.into())
                    .await
                    .map_err(IoOrbitError::from)?;
            }

            upload.complete().await.map_err(IoOrbitError::from)?;

            Ok::<(), MegaError>(())
        }
        .await;

        if res.is_err() {
            upload.abort().await.map_err(IoOrbitError::from)?;
        }

        res
    }

    /// Upload an object using a *single PUT* request.
    ///
    /// Why this method exists:
    ///
    /// object_store 0.13 changed the semantics of `ObjectStore::put`:
    /// - `put` no longer accepts a streaming body
    /// - it requires a fully-buffered `PutPayload`
    ///
    /// This helper adapts our internal `ObjectByteStream` abstraction
    /// (used throughout Mega for streaming object data)
    /// into a buffered upload suitable for backends that:
    /// - do NOT reliably support multipart upload (e.g. rustfs, some MinIO setups)
    /// - or where the object size is small enough to fit comfortably in memory
    ///
    /// Design trade-offs:
    /// - This method **buffers the entire object in memory**
    /// - It should ONLY be used for:
    ///   - small objects
    ///   - metadata-like payloads
    ///   - backends without stable multipart support
    ///
    /// For large objects (Git packfiles, LFS blobs, etc.),
    /// `put_stream` + `put_multipart` MUST be used instead.
    async fn put_single(
        &self,
        path: &object_store::path::Path,
        mut data: ObjectByteStream,
    ) -> Result<(), MegaError> {
        let mut buf = BytesMut::new();

        while let Some(chunk) = data.try_next().await? {
            buf.extend_from_slice(&chunk);
        }

        self.to_store()
            .put(path, PutPayload::from_bytes(buf.into()))
            .await
            .map_err(IoOrbitError::from)?;

        Ok(())
    }
}
