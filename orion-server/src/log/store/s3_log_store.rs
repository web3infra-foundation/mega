use anyhow::{Ok, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region},
    primitives::ByteStream,
};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::log::store::LogStore;

pub struct S3LogStore {
    client: Client,
    bucket_name: String,
    _region: Region,
}

impl S3LogStore {
    pub async fn new(
        bucket_name: &str,
        region_name: &str,
        access_key_id: &str,
        access_key: &str,
    ) -> Self {
        let region = Region::new(region_name.to_string());

        let shared_config = aws_config::defaults(BehaviorVersion::v2025_08_07())
            .region(region.clone())
            .credentials_provider(Credentials::new(
                access_key_id,
                access_key,
                None,
                None,
                "example",
            ))
            .load()
            .await;

        let client = Client::new(&shared_config);

        S3LogStore {
            client,
            bucket_name: bucket_name.to_string(),
            _region: region,
        }
    }
}

#[async_trait::async_trait]
impl LogStore for S3LogStore {
    async fn append(&self, key: &str, data: &str) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(key)
            .body(ByteStream::from(data.as_bytes().to_vec()))
            .send()
            .await?;

        Ok(())
    }

    async fn read(&self, key: &str) -> anyhow::Result<String> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await?;

        let mut body = resp.body.into_async_read();
        let mut content = String::new();
        tokio::io::AsyncReadExt::read_to_string(&mut body, &mut content).await?;

        Ok(content)
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await?;

        Ok(())
    }

    async fn read_range(&self, key: &str, start_line: usize, end_line: usize) -> Result<String> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await?;

        let body = resp.body.into_async_read();
        let reader = BufReader::new(body);
        let mut lines = reader.lines();

        let mut content = Vec::new();
        let mut line_idx = 0;

        while let Some(line) = lines.next_line().await? {
            if line_idx >= start_line && line_idx < end_line {
                content.push(line);
            }
            line_idx += 1;
            if line_idx >= end_line {
                break;
            }
        }

        Ok(content.join("\n"))
    }

    async fn log_exists(&self, key: &str) -> bool {
        self.client
            .head_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await
            .is_ok()
    }
}
