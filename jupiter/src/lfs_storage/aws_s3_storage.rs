use std::time::Duration;

use async_trait::async_trait;
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;
use bytes::Bytes;

use common::config::LFSAwsConfig;
use common::errors::{GitLFSError, MegaError};

use crate::lfs_storage::{transform_path, LfsFileStorage};

pub struct AwsS3Storage {
    client: Client,
    bucket_name: String,
    _region: Region,
}

impl AwsS3Storage {
    pub async fn init(aws_config: LFSAwsConfig) -> AwsS3Storage {
        let region_provider = RegionProviderChain::first_try(Region::new(aws_config.s3_region));
        let region = region_provider.region().await.expect("Invalid region str");

        let shared_config = aws_config::from_env()
            .region(region_provider)
            .credentials_provider(Credentials::new(
                aws_config.s3_access_key_id,
                aws_config.s3_secret_access_key,
                None,
                None,
                "example",
            ))
            .load()
            .await;

        let client = Client::new(&shared_config);
        AwsS3Storage {
            client,
            bucket_name: aws_config.s3_bucket,
            _region: region,
        }
    }
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
            .map_err(MegaError::from_s3_error);
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
            .map_err(MegaError::from_s3_error)
            .map(|_| ())
    }

    async fn put_object_with_chunk(&self, _: &str, _: &[u8], _: usize) -> Result<(), GitLFSError> {
        unimplemented!("Not Supported Yet")
    }

    async fn exist_object(&self, _: &str, _: bool) -> bool {
        true
    }
}
