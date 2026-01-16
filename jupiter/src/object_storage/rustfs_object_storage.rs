use std::{any::Any, str::FromStr};

use aws_config::{BehaviorVersion, Region, meta::region::RegionProviderChain};
use aws_sdk_s3::{
    Client,
    config::{Builder as S3ConfigBuilder, Credentials},
    primitives::{ByteStream, SdkBody},
    types::{BucketLocationConstraint, CreateBucketConfiguration},
};
use common::{config::S3Config, errors::MegaError};
use futures::StreamExt;
use http_body::Frame;
use http_body_util::StreamBody;
use tokio_util::io::ReaderStream;

use crate::object_storage::{
    ObjectByteStream, ObjectKey, ObjectMeta, ObjectStorage, dump_error_chain,
};

#[derive(Clone)]
pub struct RustfsObjectStorage {
    client: Client,
    bucket: String,
}

fn s3_key(key: &ObjectKey) -> String {
    let id = &key.key;
    format!(
        "{}/{}/{}/{}/{}",
        key.namespace,
        &id[0..2],
        &id[2..4],
        &id[4..6],
        &id[6..]
    )
}

fn object_stream_to_sdk_body(stream: ObjectByteStream) -> ByteStream {
    let body = StreamBody::new(stream.map(|res| res.map(Frame::data)));
    let sdk_body = SdkBody::from_body_1_x(body);
    ByteStream::new(sdk_body)
}

pub fn aws_byte_stream_to_object_stream(bs: ByteStream) -> ObjectByteStream {
    let reader = bs.into_async_read();
    let stream = ReaderStream::new(reader).map(|res| res.map_err(std::io::Error::other));
    Box::pin(stream)
}

impl RustfsObjectStorage {
    pub async fn new(config: S3Config) -> Result<Self, MegaError> {
        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));

        let mut builder = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .credentials_provider(Credentials::new(
                &config.access_key_id,
                &config.secret_access_key,
                None,
                None,
                "rustfs",
            ));

        if !config.endpoint_url.is_empty() {
            builder = builder.endpoint_url(&config.endpoint_url);
        }
        let shared_config = builder.load().await;

        // For S3-compatible services like MinIO, use path-style addressing
        let s3_config = if !config.endpoint_url.is_empty() {
            // Use path-style addressing for MinIO compatibility
            S3ConfigBuilder::from(&shared_config)
                .force_path_style(true)
                .build()
        } else {
            S3ConfigBuilder::from(&shared_config).build()
        };
        let client = Client::from_conf(s3_config);

        Self::create_bucket_if_not_exist(&config, client.clone()).await?;
        Ok(Self {
            client,
            bucket: config.bucket,
        })
    }

    pub(crate) async fn create_bucket_if_not_exist(
        config: &S3Config,
        client: Client,
    ) -> Result<(), MegaError> {
        let bucket_exists = client
            .head_bucket()
            .bucket(&config.bucket)
            .send()
            .await
            .is_ok();

        if !bucket_exists {
            tracing::info!("Bucket {} does not exist, creating...", &config.bucket);

            let mut req = client.create_bucket().bucket(&config.bucket);

            if config.region != "us-east-1" {
                req = req.create_bucket_configuration(
                    CreateBucketConfiguration::builder()
                        .location_constraint(
                            BucketLocationConstraint::from_str(&config.region)
                                .expect("invalid region str"),
                        )
                        .build(),
                );
            }
            match req.send().await {
                Ok(_) => {
                    tracing::info!("Bucket created successfully");
                }
                Err(e) => {
                    tracing::error!("Error creating bucket: {:?}", e);
                    let detail = dump_error_chain(&e);
                    return Err(MegaError::Other(detail));
                }
            }
        }
        Ok(())
    }
    // Helper: check if object exists in S3
    pub async fn object_exists(&self, key: &ObjectKey) -> bool {
        self.client
            .head_object()
            .bucket(&self.bucket)
            .key(s3_key(key))
            .send()
            .await
            .is_ok()
    }

    // Helper: generate a pre-signed GET url (1h default)
    pub async fn get_presigned_url(&self, key: &ObjectKey) -> Result<String, MegaError> {
        use std::time::Duration;

        use aws_sdk_s3::presigning::PresigningConfig;

        let cfg = PresigningConfig::expires_in(Duration::from_secs(3600))
            .map_err(|e| MegaError::Other(format!("Failed to create presigning config: {}", e)))?;

        let req = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(s3_key(key));

        let presigned = req
            .presigned(cfg)
            .await
            .map_err(|e| MegaError::Other(dump_error_chain(&e)))?;
        Ok(presigned.uri().to_string())
    }

    // Helper: generate a pre-signed PUT url (1h default)
    pub async fn put_presigned_url(&self, key: &ObjectKey) -> Result<String, MegaError> {
        use std::time::Duration;

        use aws_sdk_s3::presigning::PresigningConfig;

        let cfg = PresigningConfig::expires_in(Duration::from_secs(3600))
            .map_err(|e| MegaError::Other(format!("Failed to create presigning config: {}", e)))?;

        let req = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(s3_key(key));

        let presigned = req
            .presigned(cfg)
            .await
            .map_err(|e| MegaError::Other(dump_error_chain(&e)))?;
        Ok(presigned.uri().to_string())
    }
}

#[async_trait::async_trait]
impl ObjectStorage for RustfsObjectStorage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn put(
        &self,
        key: &ObjectKey,
        reader: ObjectByteStream,
        meta: ObjectMeta,
    ) -> Result<(), MegaError> {
        let body = object_stream_to_sdk_body(reader);

        let res = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(s3_key(key))
            .body(body)
            .content_length(meta.size)
            .send()
            .await;

        match res {
            Ok(_) => {}
            Err(e) => {
                let detail = dump_error_chain(&e);
                return Err(MegaError::Other(detail));
            }
        }

        Ok(())
    }

    async fn get(&self, key: &ObjectKey) -> Result<(ObjectByteStream, ObjectMeta), MegaError> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(s3_key(key))
            .send()
            .await;

        match resp {
            Ok(output) => {
                return Ok((
                    aws_byte_stream_to_object_stream(output.body),
                    ObjectMeta {
                        size: output.content_length.unwrap_or_default(),
                        ..Default::default()
                    },
                ));
            }
            Err(e) => {
                let detail = dump_error_chain(&e);
                return Err(MegaError::Other(detail));
            }
        }
    }

    async fn exists(&self, key: &ObjectKey) -> Result<bool, MegaError> {
        Ok(self.object_exists(key).await)
    }

    async fn presign_get(&self, key: &ObjectKey) -> Result<Option<String>, MegaError> {
        self.get_presigned_url(key).await.map(Some)
    }

    async fn presign_put(&self, key: &ObjectKey) -> Result<Option<String>, MegaError> {
        self.put_presigned_url(key).await.map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object_storage::ObjectNamespace;

    #[test]
    fn test_s3_key_lfs() {
        let key = ObjectKey {
            namespace: ObjectNamespace::Lfs,
            key: "abcdef1234567890".to_string(),
        };

        let result = s3_key(&key);

        // Unified 3-level sharding for LFS objects.
        assert_eq!(result, "lfs/ab/cd/ef/1234567890");
    }

    #[test]
    fn test_s3_key_git() {
        let key = ObjectKey {
            namespace: ObjectNamespace::Git,
            key: "abcdef1234567890".to_string(),
        };

        let result = s3_key(&key);

        assert_eq!(result, "git/ab/cd/ef/1234567890");
    }
}
