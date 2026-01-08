use std::{any::Any, path::PathBuf};

use bytes::Bytes;
use futures::{Stream, StreamExt};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
};

use common::errors::MegaError;

use crate::object_storage::{
    ObjectByteStream, ObjectKey, ObjectMeta, ObjectNamespace, ObjectStorage,
};

#[derive(Clone)]
pub struct FsObjectStorage {
    root: PathBuf,
}

impl FsObjectStorage {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn shard_path(namespace: &ObjectNamespace, key: &str) -> PathBuf {
        let namespace_path = match namespace {
            ObjectNamespace::Git => "git",
            ObjectNamespace::Lfs => "lfs",
            ObjectNamespace::Log => "log",
        };

        Path::new(namespace_path)
            .join(&key[0..2])
            .join(&key[2..4])
            .join(&key[4..6])
            .join(&key[6..])
    }

    fn object_path(&self, key: &ObjectKey) -> PathBuf {
        self.root.join(Self::shard_path(&key.namespace, &key.key))
    }

    /// Check if an object exists without opening the file.
    /// This is more efficient than using `get()` as it only checks metadata.
    pub async fn object_exists(&self, key: &ObjectKey) -> bool {
        let path = self.object_path(key);
        fs::metadata(&path).await.is_ok()
    }
}

fn file_stream(mut file: fs::File) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send {
    async_stream::try_stream! {
        let mut buf = vec![0u8; 8 * 1024];

        loop {
            let n = file.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            yield Bytes::copy_from_slice(&buf[..n]);
        }
    }
}

#[async_trait::async_trait]
impl ObjectStorage for FsObjectStorage {
    fn as_any(&self) -> &dyn Any {
        self
    }
    async fn put(
        &self,
        key: &ObjectKey,
        mut reader: ObjectByteStream,
        _: ObjectMeta,
    ) -> Result<(), MegaError> {
        let path = self.object_path(key);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(&path).await?;

        while let Some(chunk) = reader.next().await {
            let bytes = chunk?;
            file.write_all(&bytes).await?;
        }

        file.flush().await?;
        Ok(())
    }

    async fn get(&self, key: &ObjectKey) -> Result<(ObjectByteStream, ObjectMeta), MegaError> {
        let path = self.object_path(key);
        let file = fs::File::open(&path).await?;
        let meta_fs = file.metadata().await?;

        let size = meta_fs.len();

        let stream = file_stream(file);

        Ok((
            Box::pin(stream),
            ObjectMeta {
                size: size as i64,
                ..Default::default()
            },
        ))
    }

    async fn exists(&self, key: &ObjectKey) -> Result<bool, MegaError> {
        Ok(self.object_exists(key).await)
    }

    async fn presign_get(&self, _key: &ObjectKey) -> Result<Option<String>, MegaError> {
        Ok(None)
    }

    async fn presign_put(&self, _key: &ObjectKey) -> Result<Option<String>, MegaError> {
        Ok(None)
    }
}
