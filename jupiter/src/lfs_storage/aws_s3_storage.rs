use std::time::Duration;

use async_trait::async_trait;
use aws_config::Region;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::presigning::PresigningConfig;
use bytes::Bytes;

use common::config::S3Config;
use common::errors::MegaError;

use crate::lfs_storage::{LfsFileStorage, transform_path};

pub struct AwsS3Storage {
    client: Client,
    bucket_name: String,
    _region: Region,
}

impl AwsS3Storage {
    pub async fn init(config: S3Config) -> AwsS3Storage {
        let region_provider = RegionProviderChain::first_try(Region::new(config.region));
        let region = region_provider.region().await.expect("Invalid region str");

        let shared_config = aws_config::from_env()
            .region(region_provider)
            .credentials_provider(Credentials::new(
                config.access_key_id,
                config.secret_access_key,
                None,
                None,
                "example",
            ))
            .load()
            .await;

        let client = Client::new(&shared_config);
        AwsS3Storage {
            client,
            bucket_name: config.bucket,
            _region: region,
        }
    }
}

fn convert_s3_error<E: ProvideErrorMetadata>(err: &E) -> MegaError {
    MegaError::Other(format!(
        "Error Message: {:?}",
        err.message().map(String::from)
    ))
}

#[async_trait]
impl LfsFileStorage for AwsS3Storage {
    async fn get_object(&self, _: &str) -> Result<Bytes, MegaError> {
        // let key = format!("{}/{}", "objects", self.transform_path(object_id));
        // let res = self
        //     .client
        //     .get_object()
        //     .bucket(self.bucket_name.clone())
        //     .key(key)
        //     .send()
        //     .await
        //     .map_err(MegaError::from_s3_error);
        unimplemented!("Aws_s3 mode using presigned url instead direct download")
    }

    async fn download_url(&self, object_id: &str, _: &str) -> Result<String, MegaError> {
        let key = format!("{}/{}", "objects", transform_path(object_id));
        let expires_in = Duration::from_secs(3600);
        let presigned_request = self
            .client
            .get_object()
            .bucket(self.bucket_name.clone())
            .key(key)
            .presigned(PresigningConfig::expires_in(expires_in).unwrap())
            .await
            .map_err(|e| convert_s3_error(&e));
        Ok(presigned_request.unwrap().uri().to_string())
    }

    async fn put_object(&self, object_id: &str, body_content: Vec<u8>) -> Result<(), MegaError> {
        let body = aws_sdk_s3::primitives::ByteStream::from(body_content);
        let key = format!("{}/{}", "objects", transform_path(object_id));
        self.client
            .put_object()
            .bucket(self.bucket_name.clone())
            .key(key)
            .body(body)
            .send()
            .await
            .map_err(|e| convert_s3_error(&e))
            .map(|_| ())
    }

    async fn exist_object(&self, _: &str) -> bool {
        true
    }
}
