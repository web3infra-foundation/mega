
use anyhow::Result;

use futures::{stream, StreamExt, TryStreamExt};
use io_orbit::{
    factory::MegaObjectStorageWrapper,
    object_storage::{ObjectKey, ObjectNamespace},
};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio_util::io::StreamReader;
use tokio::io::AsyncReadExt;
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
    async fn stream_to_string(
        &self,
        mut stream: io_orbit::object_storage::ObjectByteStream,
    ) -> Result<String> {
        let mut buf = Vec::new();
        while let Some(chunk) = stream.try_next().await? {
            buf.extend_from_slice(&chunk);
        }
        String::from_utf8(buf).map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))
    }

    /// Convert a string to a byte stream.
    fn string_to_stream(&self, content: String) -> io_orbit::object_storage::ObjectByteStream {
        Box::pin(stream::once(async move {
            Ok::<Bytes, std::io::Error>(Bytes::from(content))
        }))
    }

    /// Convert ObjectByteStream to AsyncRead for line-by-line reading.
    fn stream_to_async_read(
        &self,
        stream: io_orbit::object_storage::ObjectByteStream,
    ) -> impl AsyncRead {
        StreamReader::new(stream.map_err(|e| std::io::Error::other(e)))
    }
}

#[async_trait::async_trait]
impl LogStore for IoOrbitLogStore {
    async fn append(&self, key: &str, data: &str) -> Result<()> {
        let obj_key = self.to_object_key(key);

        // Read existing content (if exists)
        let existing_content = match self.storage.inner.get_stream(&obj_key).await {
            Ok((stream, _)) => self.stream_to_string(stream).await.unwrap_or_default(),
            Err(_) => String::new(), // File doesn't exist, create new
        };

        // Append new data
        let new_content = if existing_content.is_empty() {
            data.to_string()
        } else if existing_content.ends_with('\n') {
            format!("{}{}", existing_content, data)
        } else {
            format!("{}\n{}", existing_content, data)
        };

        // Write complete content
        let stream = self.string_to_stream(new_content);
        self.storage
            .inner
            .put_stream(&obj_key, stream, Default::default())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to append log: {}", e))?;

        Ok(())
    }

    async fn read(&self, key: &str) -> Result<String> {
        let obj_key = self.to_object_key(key);

        let (stream, _) = self
            .storage
            .inner
            .get_stream(&obj_key)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read log: {}", e))?;

        self.stream_to_string(stream).await
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let obj_key = self.to_object_key(key);
        self.storage.inner.delete(&obj_key).await.map_err(|e| anyhow::anyhow!("Failed to delete log: {}", e))?;

        Ok(())
    }

    async fn read_range(
        &self,
        key: &str,
        start_line: usize,
        end_line: usize,
    ) -> Result<String> {
        let obj_key = self.to_object_key(key);
        if end_line <= start_line {
            return Ok(String::new());
        }

        // Use fixed-size byte windows (Range) and count lines while streaming.
        // No "line length estimation" is used.
        const CHUNK_SIZE: u64 = 1024 * 1024; // 1 MiB per request

        let mut offset: u64 = 0;
        let mut carry = String::new(); // partial line carried across chunks
        let mut line_idx: usize = 0;
        let mut out: Vec<String> = Vec::new();

        loop {
            let (stream, _) = self
                .storage
                .inner
                .get_range_stream(&obj_key, offset, Some(offset.saturating_add(CHUNK_SIZE)))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to read log range: {}", e))?;

            // Buffer this chunk (range is already bounded, so this is safe)
            let mut reader = self.stream_to_async_read(stream);
            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).await?;

            if buf.is_empty() {
                break; // EOF
            }

            offset = offset.saturating_add(buf.len() as u64);

            // Decode as UTF-8; logs are expected to be text.
            let chunk = String::from_utf8_lossy(&buf);
            let mut combined = String::new();
            combined.push_str(&carry);
            combined.push_str(&chunk);

            // If the chunk does not end with '\n', keep the last partial line in `carry`.
            carry.clear();
            let ends_with_newline = combined.as_bytes().last().copied() == Some(b'\n');

            let mut parts = combined.split('\n').peekable();
            while let Some(part) = parts.next() {
                let is_last = parts.peek().is_none();
                if is_last && !ends_with_newline {
                    carry.push_str(part);
                    break;
                }

                // `part` is a complete line (without '\n')
                if line_idx >= start_line && line_idx < end_line {
                    out.push(part.to_string());
                    if out.len() >= (end_line - start_line) {
                        return Ok(out.join("\n"));
                    }
                }
                line_idx += 1;
            }

            // If we got less than CHUNK_SIZE, we're at EOF.
            if (buf.len() as u64) < CHUNK_SIZE {
                break;
            }
        }

        // If file ends without '\n', there may be one last line in carry.
        if !carry.is_empty() && line_idx >= start_line && line_idx < end_line {
            out.push(carry);
        }

        Ok(out.join("\n"))
    }

    async fn log_exists(&self, key: &str) -> bool {
        let obj_key = self.to_object_key(key);

        self.storage.inner.exists(&obj_key).await.unwrap_or_else(|_| false)
    }
}