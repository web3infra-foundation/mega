use std::{any::Any, str::FromStr};

use aws_config::{BehaviorVersion, Region, meta::region::RegionProviderChain};
use aws_sdk_s3::{
    Client,
    config::Credentials,
    primitives::{ByteStream, SdkBody},
    types::{BucketLocationConstraint, CreateBucketConfiguration},
};
use futures::StreamExt;
use http_body::Frame;
use http_body_util::StreamBody;
use tokio_util::io::ReaderStream;

use common::{config::S3Config, errors::MegaError};

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
    let shard1 = &id[0..2];
    let shard2 = &id[2..4];
    let shard3 = &id[4..6];

    format!(
        "{}/{}/{}/{}/{}",
        key.namespace,
        shard1,
        shard2,
        shard3,
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

        let client = Client::new(&shared_config);

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
}

#[cfg(test)]
mod tests {
    use crate::object_storage::ObjectNamespace;

    use super::*;

    #[test]
    fn test_s3_key_basic() {
        let key = ObjectKey {
            namespace: ObjectNamespace::Git,
            key: "abcdef1234567890".to_string(),
        };

        let result = s3_key(&key);

        assert_eq!(result, "git/ab/cd/ef/1234567890");
    }
}
