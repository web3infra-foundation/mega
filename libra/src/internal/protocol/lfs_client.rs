use reqwest::Client;
use url::Url;
use ceres::lfs::lfs_structs::{BatchRequest, RequestVars};
use mercury::internal::object::types::ObjectType;
use mercury::internal::pack::entry::Entry;
use crate::internal::protocol::https_client::BasicAuth;
use crate::internal::protocol::ProtocolClient;
use crate::utils::lfs;

pub struct LFSClient {
    pub url: Url,
    pub client: Client,
}

impl ProtocolClient for LFSClient {
    /// Construct LFSClient from a given Repo URL.
    fn from_url(repo_url: &Url) -> Self {
        let lfs_server = Url::parse(&lfs::generate_lfs_server_url(repo_url.to_string())).unwrap();
        let client = Client::builder()
            .http1_only()
            .default_headers(lfs::LFS_HEADERS.clone())
            .build()
            .unwrap();
        Self {
            url: lfs_server.join("/objects/batch").unwrap(),
            client,
        }
    }
}

impl LFSClient {
    /// push LFS objects to remote server
    pub async fn push_objects<'a, I>(&self, objs: I, auth: Option<BasicAuth>)
    where
        I: IntoIterator<Item = &'a Entry>
    {
        // filter pointer file within blobs
        let mut lfs_oids = Vec::new();
        for blob in objs.into_iter().filter(|e| e.obj_type == ObjectType::Blob) {
            let oid = lfs::parse_pointer_data(&blob.data);
            if let Some(oid) = oid {
                lfs_oids.push(oid);
            }
        }

        let mut lfs_objs = Vec::new();
        for oid in &lfs_oids {
            let path = lfs::lfs_object_path(oid);
            if !path.exists() {
                eprintln!("fatal: LFS object not found: {}", oid);
                return;
            }
            let size = path.metadata().unwrap().len() as i64;
            lfs_objs.push(RequestVars {
                oid: oid.to_owned(),
                size,
                ..Default::default()
            })
        }

        let batch_request = BatchRequest {
            operation: "upload".to_string(),
            transfers: vec![lfs::LFS_TRANSFER_API.to_string()],
            objects: lfs_objs,
            hash_algo: lfs::LFS_HASH_ALGO.to_string(),
            enable_split: None,
        };

        let mut request = self.client.post(self.url.clone()).json(&batch_request);
        if let Some(auth) = auth {
            request = request.basic_auth(auth.username, Some(auth.password));
        }

        let response = request.send().await.unwrap();
        tracing::debug!("LFS push response: {:?}", response.json::<serde_json::Value>().await.unwrap());
    }
}