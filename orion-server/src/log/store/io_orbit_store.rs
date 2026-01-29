use anyhow::Result;
use futures::{TryStreamExt, stream};
use io_orbit::{
    factory::MegaObjectStorageWrapper,
    object_storage::{ObjectByteStream, ObjectKey, ObjectMeta, ObjectNamespace},
};
use tokio_util::bytes::Bytes;

use crate::log::store::LogStore;

pub struct IoOrbitLogStore {
    storage: MegaObjectStorageWrapper,
}

impl IoOrbitLogStore {
    pub fn new(storage: MegaObjectStorageWrapper) -> Self {
        Self { storage }
    }

    /// Convert a LogStore key to an ObjectKey with Log namespace.
    fn to_object_key(&self, key: &str) -> ObjectKey {
        ObjectKey {
            namespace: ObjectNamespace::Log,
            key: key.to_string(),
        }
    }

    /// Convert a byte stream to a string.
    async fn stream_to_string(&self, mut stream: ObjectByteStream) -> Result<String> {
        let mut buf = Vec::new();
        while let Some(chunk) = stream.try_next().await? {
            buf.extend_from_slice(&chunk);
        }
        String::from_utf8(buf).map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))
    }

    /// Convert a string to a byte stream.
    fn string_to_stream(&self, content: String) -> ObjectByteStream {
        Box::pin(stream::once(async move {
            Ok::<Bytes, std::io::Error>(Bytes::from(content))
        }))
    }
}

#[async_trait::async_trait]
impl LogStore for IoOrbitLogStore {
    async fn append(&self, key: &str, data: &str) -> Result<()> {
        let obj_key = self.to_object_key(key);
        let stream = self.string_to_stream(data.to_string());

        self.storage
            .inner
            .append_concurrently(&obj_key, stream, ObjectMeta::default())
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }

    async fn read(&self, key: &str) -> Result<String> {
        let obj_key = self.to_object_key(key);

        // Use LogStorage abstraction to read complete log: read [0, len) based on manifest length.
        let manifest = self
            .storage
            .inner
            .load_manifest(&obj_key)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load log manifest: {}", e))?;

        if manifest.len == 0 {
            return Ok(String::new());
        }

        let stream = self
            .storage
            .inner
            .read_range(&obj_key, 0, manifest.len)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read log: {}", e))?;

        self.stream_to_string(stream).await
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let obj_key = self.to_object_key(key);
        self.storage
            .inner
            .delete(&obj_key)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete log: {}", e))?;

        Ok(())
    }

    async fn read_range(&self, key: &str, start_line: usize, end_line: usize) -> Result<String> {
        let obj_key = self.to_object_key(key);
        let stream = self
            .storage
            .inner
            .read_lines_range(&obj_key, start_line as u64, end_line as u64)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read log range: {}", e))?;
        self.stream_to_string(stream).await
    }

    async fn log_exists(&self, key: &str) -> bool {
        let obj_key = self.to_object_key(key);
        match self.storage.inner.log_exists(&obj_key).await {
            Ok(exists) => exists,
            Err(e) => {
                tracing::warn!(key = %key, error = %e, "Error checking log existence, treating as non-existent");
                false
            }
        }
    }
}
