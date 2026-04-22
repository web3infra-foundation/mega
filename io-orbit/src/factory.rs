use std::{
    fs::{create_dir_all, exists},
    sync::Arc,
};

use common::{
    config::{ObjectStorageBackend, ObjectStorageConfig},
    errors::MegaError,
};
use object_store::{aws::AmazonS3Builder, gcp::GoogleCloudStorageBuilder, local::LocalFileSystem};

use crate::{
    adapter::{BackendStore, ObjectStoreAdapter, UploadStrategy},
    log_storage::LogStorage,
    object_storage::MegaObjectStorage,
};

pub trait MegaObjectStorageWithLog: MegaObjectStorage + LogStorage {}

impl<T: MegaObjectStorage + LogStorage> MegaObjectStorageWithLog for T {}

#[derive(Clone)]
pub struct MegaObjectStorageWrapper {
    pub inner: Arc<dyn MegaObjectStorageWithLog>,
}

impl MegaObjectStorageWrapper {
    pub fn new(inner: Arc<dyn MegaObjectStorageWithLog>) -> Self {
        Self { inner }
    }

    pub fn supports_presigned_urls(&self) -> bool {
        MegaObjectStorage::supports_presigned_urls(&*self.inner)
    }

    pub fn mock() -> Self {
        if !exists("/tmp/mega_test_object_storage").expect("mock err") {
            create_dir_all("/tmp/mega_test_object_storage").expect("init mock file err")
        }
        let fs = LocalFileSystem::new_with_prefix("/tmp/mega_test_object_storage")
            .expect("mock init error");
        let store = BackendStore::Local(Arc::new(fs));
        let adapter = Arc::new(ObjectStoreAdapter {
            store,
            upload_strategy: UploadStrategy::SinglePut,
        });
        MegaObjectStorageWrapper::new(adapter)
    }
}

pub struct ObjectStorageFactory;

impl ObjectStorageFactory {
    /// Builds object storage from [`ObjectStorageConfig::storage_type`] and nested credentials/paths.
    pub async fn build(cfg: &ObjectStorageConfig) -> Result<MegaObjectStorageWrapper, MegaError> {
        match cfg.storage_type {
            ObjectStorageBackend::S3 => build_s3_like(cfg, false).await,
            ObjectStorageBackend::S3Compatible => build_s3_like(cfg, true).await,
            ObjectStorageBackend::Gcs => build_gcs(cfg).await,
            ObjectStorageBackend::Local => build_local(cfg).await,
        }
    }
}

/// Shared S3 / S3-compatible construction (differs only by endpoint and upload strategy).
async fn build_s3_like(
    cfg: &ObjectStorageConfig,
    compatible: bool,
) -> Result<MegaObjectStorageWrapper, MegaError> {
    let s3_cfg = cfg.s3.clone();
    let mut builder = AmazonS3Builder::new()
        .with_region(&s3_cfg.region)
        .with_bucket_name(&s3_cfg.bucket)
        .with_access_key_id(&s3_cfg.access_key_id)
        .with_secret_access_key(&s3_cfg.secret_access_key);
    if compatible {
        builder = builder
            .with_endpoint(&s3_cfg.endpoint_url)
            .with_allow_http(true)
            .with_virtual_hosted_style_request(false);
    }
    let s3 = builder
        .build()
        .map_err(|e| MegaError::Other(e.to_string()))?;

    let store = BackendStore::S3(Arc::new(s3));
    let upload_strategy = if compatible {
        UploadStrategy::SinglePut
    } else {
        UploadStrategy::Multipart
    };
    let adapter = Arc::new(ObjectStoreAdapter {
        store,
        upload_strategy,
    });

    Ok(MegaObjectStorageWrapper::new(adapter))
}

async fn build_gcs(cfg: &ObjectStorageConfig) -> Result<MegaObjectStorageWrapper, MegaError> {
    let gcp_cfg = cfg.gcs.clone();
    let gcs = GoogleCloudStorageBuilder::from_env()
        .with_bucket_name(&gcp_cfg.bucket)
        .build()
        .map_err(|e| MegaError::Other(e.to_string()))?;
    let store = BackendStore::Gcs(Arc::new(gcs));
    let adapter = Arc::new(ObjectStoreAdapter {
        store,
        upload_strategy: UploadStrategy::SinglePut,
    });

    Ok(MegaObjectStorageWrapper::new(adapter))
}

async fn build_local(cfg: &ObjectStorageConfig) -> Result<MegaObjectStorageWrapper, MegaError> {
    if !exists(&cfg.local.root_dir)? {
        create_dir_all(&cfg.local.root_dir)?
    }
    let fs = LocalFileSystem::new_with_prefix(&cfg.local.root_dir)
        .map_err(|e| MegaError::Other(e.to_string()))?;

    let store = BackendStore::Local(Arc::new(fs));
    let adapter = Arc::new(ObjectStoreAdapter {
        store,
        upload_strategy: UploadStrategy::SinglePut,
    });

    Ok(MegaObjectStorageWrapper::new(adapter))
}
