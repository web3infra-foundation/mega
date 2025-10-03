use super::ProtocolClient;
use crate::utils::lfs;
use ceres::lfs::lfs_structs::{BatchRequest, Operation, Ref, RequestObject, ResponseObject};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;
use url::Url;

#[derive(Debug)]
pub struct LFSClient {
    pub batch_url: Url,
    pub lfs_url: Url,
    pub client: Client,
}

static LFS_CLIENT: OnceCell<LFSClient> = OnceCell::const_new();

impl LFSClient {
    /// Get LFSClient instance
    pub async fn get() -> &'static LFSClient {
        LFS_CLIENT
            .get_or_init(|| async { LFSClient::new().await })
            .await
    }
}

/// see [successful-responses](https://github.com/git-lfs/git-lfs/blob/main/docs/api/batch.md#successful-responses)
#[derive(Serialize, Deserialize)]
pub struct LfsBatchResponse {
    pub transfer: Option<String>,
    pub objects: Vec<ResponseObject>,
    pub hash_algo: Option<String>,
}

impl ProtocolClient for LFSClient {
    /// Construct LFSClient from a given Repo URL.
    fn from_url(repo_url: &Url) -> Self {
        // The trailing slash is MUST, or `join()` method will replace the last segment.
        // like: Url("/info/lfs").join("objects/batch") => "/info/objects/batch"
        let lfs_server = lfs::generate_lfs_server_url(repo_url.to_string()) + "/"; // IMPORTANT
        let lfs_server = Url::parse(&lfs_server).unwrap();
        let client = Client::builder()
            .default_headers(lfs::LFS_HEADERS.clone()) //  will be overwritten by `json()`, careful!
            .build()
            .unwrap();
        Self {
            // Caution: DO NOT start with `/`, or path after domain will be replaced.
            batch_url: lfs_server.join("objects/batch").unwrap(),
            lfs_url: lfs_server,
            client,
        }
    }
}

impl LFSClient {
    /// Construct LFSClient from a given URL string.
    pub async fn new() -> Self {
        // TODO: Replace with proper remote URL detection for scorpio
        let url = "http://localhost:8000".to_string();
        LFSClient::from_url(&Url::parse(&url).unwrap())
    }

    /// Create a new LFSClient with a specific mono path
    pub fn new_with_path(mono_path: &str) -> Self {
        let url = format!("http://localhost:8000/{}", mono_path);
        LFSClient::from_url(&Url::parse(&url).unwrap())
    }

    /// Get batch response for upload operation
    pub async fn batch_upload(
        &self,
        _refs: Vec<Ref>,
        objects: Vec<RequestObject>,
    ) -> Result<LfsBatchResponse, Box<dyn std::error::Error>> {
        let batch_request = BatchRequest {
            operation: Operation::Upload,
            transfers: vec![lfs::LFS_TRANSFER_API.to_string()],
            objects,
            hash_algo: lfs::LFS_HASH_ALGO.to_string(),
        };

        let response = self
            .client
            .post(self.batch_url.clone())
            .json(&batch_request)
            .send()
            .await?;

        if response.status().is_success() {
            let batch_response: LfsBatchResponse = response.json().await?;
            Ok(batch_response)
        } else {
            Err(format!("Batch upload failed with status: {}", response.status()).into())
        }
    }

    /// Get batch response for download operation
    pub async fn batch_download(
        &self,
        _refs: Vec<Ref>,
        objects: Vec<RequestObject>,
    ) -> Result<LfsBatchResponse, Box<dyn std::error::Error>> {
        let batch_request = BatchRequest {
            operation: Operation::Download,
            transfers: vec![lfs::LFS_TRANSFER_API.to_string()],
            objects,
            hash_algo: lfs::LFS_HASH_ALGO.to_string(),
        };

        let response = self
            .client
            .post(self.batch_url.clone())
            .json(&batch_request)
            .send()
            .await?;

        if response.status().is_success() {
            let batch_response: LfsBatchResponse = response.json().await?;
            Ok(batch_response)
        } else {
            Err(format!("Batch download failed with status: {}", response.status()).into())
        }
    }

    /// Upload a file to LFS server
    pub async fn upload_file(
        &self,
        oid: &str,
        data: Vec<u8>,
        upload_url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response = self.client.put(upload_url).body(data).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("Upload failed for oid {}: {}", oid, response.status()).into())
        }
    }

    /// Download a file from LFS server
    pub async fn download_file(
        &self,
        download_url: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let response = self.client.get(download_url).send().await?;

        if response.status().is_success() {
            let data = response.bytes().await?;
            Ok(data.to_vec())
        } else {
            Err(format!("Download failed: {}", response.status()).into())
        }
    }

    /// Download LFS object (simplified version)
    pub async fn download_object(
        &self,
        oid: &str,
        size: u64,
        _path: impl AsRef<std::path::Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let batch_request = BatchRequest {
            operation: Operation::Download,
            transfers: vec![lfs::LFS_TRANSFER_API.to_string()],
            objects: vec![RequestObject {
                oid: oid.to_owned(),
                size: size as i64,
                ..Default::default()
            }],
            hash_algo: lfs::LFS_HASH_ALGO.to_string(),
        };

        let response = self
            .client
            .post(self.batch_url.clone())
            .json(&batch_request)
            .send()
            .await?;

        if response.status().is_success() {
            let _batch_response: LfsBatchResponse = response.json().await?;
            // Simplified implementation - in a real implementation, you would
            // use the download URLs from the batch response
            Ok(())
        } else {
            Err(format!("Download object failed: {}", response.status()).into())
        }
    }

    /// Upload LFS object (simplified version)
    pub async fn upload_object(
        &self,
        oid: &str,
        size: u64,
        _data: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let batch_request = BatchRequest {
            operation: Operation::Upload,
            transfers: vec![lfs::LFS_TRANSFER_API.to_string()],
            objects: vec![RequestObject {
                oid: oid.to_owned(),
                size: size as i64,
                ..Default::default()
            }],
            hash_algo: lfs::LFS_HASH_ALGO.to_string(),
        };

        let response = self
            .client
            .post(self.batch_url.clone())
            .json(&batch_request)
            .send()
            .await?;

        if response.status().is_success() {
            let _batch_response: LfsBatchResponse = response.json().await?;
            // Simplified implementation - in a real implementation, you would
            // use the upload URLs from the batch response
            Ok(())
        } else {
            Err(format!("Upload object failed: {}", response.status()).into())
        }
    }

    /// Get locks (stub implementation)
    pub async fn get_locks(
        &self,
        _query: ceres::lfs::lfs_structs::LockListQuery,
    ) -> Result<ceres::lfs::lfs_structs::LockList, Box<dyn std::error::Error>> {
        // Stub implementation
        Ok(ceres::lfs::lfs_structs::LockList {
            locks: vec![],
            next_cursor: String::new(),
        })
    }

    /// Lock file (stub implementation)
    pub async fn lock(
        &self,
        _path: String,
        _ref_name: Option<String>,
    ) -> Result<ceres::lfs::lfs_structs::LockRequest, Box<dyn std::error::Error>> {
        // Stub implementation - return error
        Err("Lock not implemented".into())
    }

    /// Unlock file (stub implementation)
    pub async fn unlock(
        &self,
        _id: String,
        _force: bool,
        _ref_name: Option<String>,
    ) -> Result<ceres::lfs::lfs_structs::UnlockRequest, Box<dyn std::error::Error>> {
        // Stub implementation - return error
        Err("Unlock not implemented".into())
    }

    /// Verify locks (stub implementation)
    pub async fn verify_locks(
        &self,
        _request: ceres::lfs::lfs_structs::VerifiableLockRequest,
    ) -> Result<(u16, ceres::lfs::lfs_structs::VerifiableLockList), Box<dyn std::error::Error>>
    {
        // Stub implementation
        Ok((
            200,
            ceres::lfs::lfs_structs::VerifiableLockList {
                ours: vec![],
                theirs: vec![],
                next_cursor: String::new(),
            },
        ))
    }
}
