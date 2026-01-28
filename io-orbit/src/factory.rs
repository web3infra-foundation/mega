use std::{
    fs::{create_dir, exists},
    sync::Arc,
};

use common::{
    config::{ObjectStorageBackend, ObjectStorageConfig},
    errors::MegaError,
};
use aws_config::BehaviorVersion;
use object_store::{aws::AmazonS3Builder, gcp::GoogleCloudStorageBuilder, local::LocalFileSystem};

use crate::{
    adapter::{BackendStore, ObjectStoreAdapter, UploadStrategy},
    object_storage::MegaObjectStorage,
};

#[derive(Clone)]
pub struct MegaObjectStorageWrapper {
    pub inner: Arc<dyn MegaObjectStorage>,
}

impl MegaObjectStorageWrapper {
    pub fn new(inner: Arc<dyn MegaObjectStorage>) -> Self {
        Self { inner }
    }

    pub fn mock() -> Self {
        if !exists("/tmp/mega_test_object_storage").expect("mock err") {
            create_dir("/tmp/mega_test_object_storage").expect("init mock file err")
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
    pub async fn build(
        backend: ObjectStorageBackend,
        cfg: &ObjectStorageConfig,
    ) -> Result<MegaObjectStorageWrapper, MegaError> {
        match backend {
            ObjectStorageBackend::S3 => build_s3(cfg).await,
            ObjectStorageBackend::S3Compatible => build_s3_compatible(cfg).await,
            ObjectStorageBackend::Gcs => build_gcs(cfg).await,
            ObjectStorageBackend::Local => build_local(cfg).await,
        }
    }
}

/// Best-effort helper to ensure the configured S3-compatible bucket exists.
/// Used for local RustFS demo so we don't depend on external scripts or AWS CLI.
async fn ensure_bucket_exists_s3_compatible(cfg: &ObjectStorageConfig) -> Result<(), MegaError> {
    let s3_cfg = cfg.s3.clone();

    // If bucket or endpoint is missing, skip silently.
    if s3_cfg.bucket.is_empty() || s3_cfg.endpoint_url.is_empty() {
        tracing::warn!(
            "object_storage.s3.bucket or endpoint_url is empty; skipping automatic bucket creation"
        );
        return Ok(());
    }

    let region = aws_sdk_s3::config::Region::new(s3_cfg.region.clone());
    let credentials = aws_sdk_s3::config::Credentials::new(
        s3_cfg.access_key_id.clone(),
        s3_cfg.secret_access_key.clone(),
        None,
        None,
        "mega-config",
    );

    let shared_conf = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .endpoint_url(s3_cfg.endpoint_url.clone())
        .credentials_provider(credentials)
        .load()
        .await;

    let client = aws_sdk_s3::Client::new(&shared_conf);

    tracing::info!(
        "Ensuring S3-compatible bucket '{}' exists at {}",
        s3_cfg.bucket,
        s3_cfg.endpoint_url
    );

    match client.create_bucket().bucket(&s3_cfg.bucket).send().await {
        Ok(_) => {
            tracing::info!("Created S3-compatible bucket '{}'", s3_cfg.bucket);
        }
        Err(e) => {
            let msg = format!("{e}");
            if msg.contains("BucketAlreadyOwnedByYou")
                || msg.contains("BucketAlreadyExists")
                || msg.contains("Conflict")
            {
                tracing::info!(
                    "Bucket '{}' already exists or is owned by us; continuing. ({msg})",
                    s3_cfg.bucket
                );
            } else {
                tracing::warn!(
                    "Failed to create S3-compatible bucket '{}': {msg}. \
                     Proceeding anyway; writes may fail if the bucket truly does not exist.",
                    s3_cfg.bucket
                );
            }
        }
    }

    Ok(())
}

async fn build_s3(cfg: &ObjectStorageConfig) -> Result<MegaObjectStorageWrapper, MegaError> {
    let s3_cfg = cfg.s3.clone();
    let s3 = AmazonS3Builder::new()
        .with_region(&s3_cfg.region)
        .with_bucket_name(&s3_cfg.bucket)
        .with_access_key_id(&s3_cfg.access_key_id)
        .with_secret_access_key(&s3_cfg.secret_access_key)
        .build()
        .map_err(|e| MegaError::Other(e.to_string()))?;

    let store = BackendStore::S3(Arc::new(s3));
    let adapter = Arc::new(ObjectStoreAdapter {
        store,
        upload_strategy: UploadStrategy::Multipart,
    });

    Ok(MegaObjectStorageWrapper::new(adapter))
}

async fn build_s3_compatible(
    cfg: &ObjectStorageConfig,
) -> Result<MegaObjectStorageWrapper, MegaError> {
    let s3_cfg = cfg.s3.clone();

    // Best-effort bucket creation for local demo / RustFS.
    if let Err(e) = ensure_bucket_exists_s3_compatible(cfg).await {
        tracing::warn!(
            "Error while ensuring S3-compatible bucket exists (ignored for demo): {}",
            e
        );
    }

    let s3 = AmazonS3Builder::new()
        .with_region(&s3_cfg.region)
        .with_bucket_name(&s3_cfg.bucket)
        .with_access_key_id(&s3_cfg.access_key_id)
        .with_secret_access_key(&s3_cfg.secret_access_key)
        .with_endpoint(&s3_cfg.endpoint_url)
        .with_allow_http(true)
        .with_virtual_hosted_style_request(false)
        .build()
        .map_err(|e| MegaError::Other(e.to_string()))?;

    let store = BackendStore::S3(Arc::new(s3));
    let adapter = Arc::new(ObjectStoreAdapter {
        store,
        upload_strategy: UploadStrategy::SinglePut,
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
        create_dir(&cfg.local.root_dir)?
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
