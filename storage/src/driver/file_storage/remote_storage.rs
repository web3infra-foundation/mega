use std::env;

use crate::driver::file_storage::s3_service;
use async_trait::async_trait;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{
    config::{Credentials, Region},
    Client,
};
use bytes::Bytes;
use common::errors::MegaError;

use super::FileStorage;

pub struct RemoteStorage {
    pub region: Region,
    pub client: Client,
    pub bucket_name: String,
}

impl RemoteStorage {
    pub async fn init(bucket_name: String) -> RemoteStorage {
        let region = env::var("MEGA_OBJ_REMOTE_REGION").unwrap();
        let endpoint = env::var("MEGA_OBJ_REMOTE_ENDPOINT").unwrap();

        let region_provider = RegionProviderChain::first_try(Region::new(region));
        let region = region_provider.region().await.unwrap();

        let shared_config = aws_config::from_env()
            .region(region_provider)
            .credentials_provider(Credentials::new(
                "AK",
                "SK",
                None,
                None,
                "mega",
            ))
            .endpoint_url(endpoint)
            .load()
            .await;

        let client = Client::new(&shared_config);
        s3_service::create_bucket(&client, &bucket_name, region.as_ref())
            .await
            .unwrap();
        RemoteStorage {
            region,
            client,
            bucket_name,
        }
    }
}

#[async_trait]
impl FileStorage for RemoteStorage {
    async fn get(&self, object_id: &str) -> Result<Bytes, MegaError> {
        let key = self.transform_path(object_id);
        let res = s3_service::download_object(&self.client, &self.bucket_name, &key)
            .await
            .unwrap();
        let data = res.body.collect().await.expect("error reading data");
        Ok(data.into_bytes())
    }

    async fn put(
        &self,
        object_id: &str,
        _size: i64,
        body_content: &[u8],
    ) -> Result<String, MegaError> {
        let key = self.transform_path(object_id);
        s3_service::upload_object_from_content(
            &self.client,
            &self.bucket_name,
            body_content,
            &key,
        )
        .await
        .unwrap();
        let url = format!(
            "https://{}.obs.{}.myhuaweicloud.com/{}",
            self.bucket_name,
            self.region.as_ref(),
            key
        );
        Ok(url)
    }

    fn exist(&self, _object_id: &str) -> bool {
        todo!()
    }
}
