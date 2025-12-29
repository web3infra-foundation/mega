use std::{any::Any, path::PathBuf};

use bytes::Bytes;
use futures::{Stream, StreamExt, TryStreamExt};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
};

use common::errors::MegaError;

use crate::object_storage::{
    MultiObjectByteStream, ObjectByteStream, ObjectKey, ObjectMeta, ObjectNamespace, ObjectStorage,
};

#[derive(Clone)]
pub struct FsObjectStorage {
    root: PathBuf,
}

impl FsObjectStorage {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn object_path(&self, key: &ObjectKey) -> PathBuf {
        self.root
            .join(match key.namespace {
                ObjectNamespace::Git => "git",
                ObjectNamespace::Lfs => "lfs",
                ObjectNamespace::Log => "log",
            })
            .join(&key.key)
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

    async fn put_many(
        &self,
        objects: MultiObjectByteStream<'_>,
        concurrency: usize,
    ) -> Result<(), MegaError> {
        objects
            .try_for_each_concurrent(concurrency, |(key, stream, meta)| async move {
                self.put(&key, stream, meta).await?;
                Ok(())
            })
            .await
    }
    fn get_many(&self, keys: Vec<ObjectKey>, concurrency: usize) -> MultiObjectByteStream<'_> {
        Box::pin(
            futures::stream::iter(keys)
                .map(move |key| async move {
                    let (stream, meta) = self.get(&key).await?;
                    Ok((key, stream, meta))
                })
                .buffer_unordered(concurrency),
        )
    }
}
