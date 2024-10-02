use crate::command;
use crate::internal::config::Config;
use crate::internal::protocol::https_client::BasicAuth;
use crate::internal::protocol::ProtocolClient;
use crate::utils::lfs;
use async_static::async_static;
use ceres::lfs::lfs_structs::{BatchRequest, FetchchunkResponse, Link, LockList, LockListQuery, LockRequest, Ref, Representation, RequestVars, UnlockRequest, VerifiableLockList, VerifiableLockRequest};
use futures_util::StreamExt;
use mercury::internal::object::types::ObjectType;
use mercury::internal::pack::entry::Entry;
use reqwest::{Client, StatusCode};
use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use url::Url;

async_static! {
    pub static ref LFS_CLIENT: LFSClient = LFSClient::new().await;
}

pub struct LFSClient {
    pub batch_url: Url,
    pub lfs_url: Url,
    pub client: Client,
}

/// see [successful-responses](https://github.com/git-lfs/git-lfs/blob/main/docs/api/batch.md#successful-responses)
#[derive(Serialize, Deserialize)]
struct LfsBatchResponse {
    transfer: Option<String>,
    objects: Vec<Representation>,
    hash_algo: Option<String>,
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
    /// Construct LFSClient from current remote URL.
    pub async fn new() -> Self {
        let url = Config::get_current_remote_url().await;
        match url {
            Some(url) => LFSClient::from_url(&Url::parse(&url).unwrap()),
            None => panic!("fatal: no remote set for current branch, use `libra branch --set-upstream-to <remote>/<branch>`"),
        }
    }

    /// push LFS objects to remote server
    pub async fn push_objects<'a, I>(&self, objs: I, auth: Option<BasicAuth>) -> Result<(), ()>
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
        for (oid, _) in &lfs_oids {
            let path = lfs::lfs_object_path(oid);
            if !path.exists() {
                eprintln!("fatal: LFS object not found: {}", oid);
                continue;
            }
            let size = path.metadata().unwrap().len() as i64;
            lfs_objs.push(RequestVars {
                oid: oid.to_owned(),
                size,
                ..Default::default()
            })
        }

        if lfs_objs.is_empty() {
            tracing::info!("No LFS objects to push.");
            return Ok(());
        }

        { // verify locks
            let (code, locks) = self.verify_locks(VerifiableLockRequest {
                refs: Ref { name: command::lfs::current_refspec().await.unwrap() },
                ..Default::default()
            }, auth.clone()).await;

            if code == StatusCode::FORBIDDEN {
                eprintln!("fatal: Forbidden: You must have push access to verify locks");
                return Err(());
            } else if code == StatusCode::NOT_FOUND {
                // By default, an LFS server that doesn't implement any locking endpoints should return 404.
                // This response will not halt any Git pushes.
            } else if !code.is_success() {
                eprintln!("fatal: LFS verify locks failed. Status: {}", code);
                return Err(());
            } else {
                // success
                tracing::debug!("LFS verify locks response:\n {:?}", locks);
                let oids: HashSet<String> = lfs_oids.iter().map(|(oid, _)| oid.clone()).collect();
                let ours = locks.ours.iter().filter(|l| {
                    let oid = lfs::get_oid_by_path(&l.path);
                    oids.contains(&oid)
                }).collect::<Vec<_>>();
                if !ours.is_empty() {
                    println!("The following files are locked by you, consider unlocking them:");
                    for lock in ours {
                        println!("  - {}", lock.path);
                    }
                }
                let theirs = locks.theirs.iter().filter(|l| {
                    let oid = lfs::get_oid_by_path(&l.path);
                    oids.contains(&oid)
                }).collect::<Vec<_>>();
                if !theirs.is_empty() {
                    eprintln!("Locking failed: The following files are locked by another user:");
                    for lock in theirs {
                        eprintln!("  - {}", lock.path);
                    }
                    return Err(());
                }
            }
        }

        let batch_request = BatchRequest {
            operation: "upload".to_string(),
            transfers: vec![lfs::LFS_TRANSFER_API.to_string()],
            objects: lfs_objs,
            hash_algo: lfs::LFS_HASH_ALGO.to_string(),
        };

        let mut request = self.client
            .post(self.batch_url.clone())
            .json(&batch_request)
            .headers(lfs::LFS_HEADERS.clone());
        if let Some(auth) = auth {
            request = request.basic_auth(auth.username, Some(auth.password));
        }

        let response = request.send().await.unwrap();

        let resp = response.json::<LfsBatchResponse>().await.unwrap();
        tracing::debug!("LFS push response:\n {:#?}", serde_json::to_value(&resp).unwrap());

        // TODO: parallel upload
        for obj in resp.objects {
            self.upload_object(obj).await?;
        }
        println!("LFS objects push completed.");
        Ok(())
    }

    /// upload (PUT) one LFS file to remote server
    async fn upload_object(&self, object: Representation) -> Result<(), ()> {
        if let Some(err) = object.error {
            eprintln!("fatal: LFS upload failed. Code: {}, Message: {}", err.code, err.message);
            return Err(());
        }

        if let Some(actions) = object.actions {
            let upload_link = actions.get("upload");
            if upload_link.is_none() {
                eprintln!("fatal: LFS upload failed. No upload action found");
                return Err(());
            }

            let link = upload_link.unwrap();
            let mut request = self.client.put(&link.href);
            for (k, v) in &link.header {
                request = request.header(k, v);
            }

            let file_path = lfs::lfs_object_path(&object.oid);
            let file = tokio::fs::File::open(file_path).await.unwrap();
            println!("Uploading LFS file: {}", object.oid);
            let resp = request
                .body(reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(file)))
                .send()
                .await
                .unwrap();
            if !resp.status().is_success() {
                eprintln!("fatal: LFS upload failed. Status: {}, Message: {}", resp.status(), resp.text().await.unwrap());
                return Err(());
            }
            println!("Uploaded.");
        } else {
            tracing::debug!("LFS file {} already exists on remote server", object.oid);
        }
        Ok(())
    }

    /// download (GET) one LFS file from remote server
    pub async fn download_object(&self, oid: &str, size: u64, path: impl AsRef<Path>) {
        let batch_request = BatchRequest {
            operation: "download".to_string(),
            transfers: vec![lfs::LFS_TRANSFER_API.to_string()],
            objects: vec![RequestVars {
                oid: oid.to_owned(),
                size: size as i64,
                ..Default::default()
            }],
            hash_algo: lfs::LFS_HASH_ALGO.to_string(),
        };

        let request = self.client
            .post(self.batch_url.clone())
            .json(&batch_request)
            .headers(lfs::LFS_HEADERS.clone());
        let response = request.send().await.unwrap();

        let text = response.text().await.unwrap();
        tracing::debug!("LFS download response:\n {:#?}", serde_json::from_str::<serde_json::Value>(&text).unwrap());
        let resp = serde_json::from_str::<LfsBatchResponse>(&text).unwrap();

        let link = resp.objects[0].actions.as_ref().unwrap().get("download").unwrap();

        let mut is_chunked = false;
        // Chunk API
        let links = match self.fetch_chunk_links(&link.href).await {
            Ok(chunks) => {
                is_chunked = true;
                tracing::info!("LFS Chunk API supported.");
                chunks
            },
            Err(_) => vec![link.clone()],
        };

        let mut file = tokio::fs::File::create(path).await.unwrap();
        let mut checksum = Context::new(&SHA256);
        println!("Downloading LFS file: {}", oid);
        let mut cnt = 0;
        let total = links.len();
        for link in links {
            cnt += 1;
            if is_chunked {
                println!("- part: {}/{}", cnt, total);
            }

            let mut request = self.client.get(&link.href);
            for (k, v) in &link.header {
                request = request.header(k, v);
            }

            let response = request.send().await.unwrap();
            if !response.status().is_success() {
                eprintln!("fatal: LFS download failed. Status: {}, Message: {}", response.status(), response.text().await.unwrap());
                return;
            }

            let mut stream = response.bytes_stream();

            while let Some(chunk) = stream.next().await { // TODO: progress bar
                let chunk = chunk.unwrap();
                file.write_all(&chunk).await.unwrap();
                checksum.update(&chunk);
            }
        }
        let checksum = hex::encode(checksum.finish().as_ref());
        if checksum == oid {
            println!("Downloaded.");
        } else {
            eprintln!("fatal: LFS download failed. Checksum mismatch: {} != {}. Fallback to pointer file.", checksum, oid);
            let pointer = lfs::format_pointer_string(oid, size);
            file.set_len(0).await.unwrap(); // clear
            file.write_all(pointer.as_bytes()).await.unwrap();
        }
    }

    /// Only for MonoRepo (mega)
    async fn fetch_chunk_links(&self, obj_link: &str) -> Result<Vec<Link>, ()> {
        let mut url = Url::parse(obj_link).unwrap();
        let path = url.path().trim_end_matches('/');
        url.set_path(&(path.to_owned() + "/chunks")); // reserve query params (for GitHub link)

        let request = self.client.get(url);
        let resp = request.send().await.unwrap();
        let code = resp.status();
        if code == StatusCode::NOT_FOUND || code == StatusCode::FORBIDDEN { // GitHub maybe return 403
            tracing::info!("Remote LFS Server not support Chunks API, or forbidden.");
            return Err(());
        } else if !code.is_success() {
            tracing::debug!("fatal: LFS get chunk hrefs failed. Status: {}, Message: {}", code, resp.text().await.unwrap());
            return Err(());
        }
        let mut res = resp.json::<FetchchunkResponse>().await.unwrap();
        // sort by offset
        res.chunks.sort_by(|a, b| a.offset.cmp(&b.offset));
        Ok(res.chunks.into_iter().map(|c| c.link).collect())
    }
}

// LFS locks API
impl LFSClient {
    pub async fn get_locks(&self, query: LockListQuery) -> LockList {
        let url = self.lfs_url.join("locks").unwrap();
        let mut request = self.client.get(url);
        request = request.query(&[
            ("id", query.id),
            ("path", query.path),
            ("limit", query.limit),
            ("cursor", query.cursor),
            ("refspec", query.refspec)
        ]);
        let response = request.send().await.unwrap();

        if !response.status().is_success() {
            eprintln!("fatal: LFS get locks failed. Status: {}, Message: {}", response.status(), response.text().await.unwrap());
            return LockList {
                locks: Vec::new(),
                next_cursor: String::default(),
            };
        }

        response.json::<LockList>().await.unwrap()
    }

    /// lock an LFS file
    /// - `refspec` is must in Mega Server, but optional in Git Doc
    pub async fn lock(&self, path: String, refspec: String, basic_auth: Option<BasicAuth>) -> StatusCode {
        let url = self.lfs_url.join("locks").unwrap();
        let mut request = self.client.post(url).json(&LockRequest {
            path,
            refs: Ref { name: refspec },
        });
        if let Some(auth) = basic_auth {
            request = request.basic_auth(auth.username, Some(auth.password));
        }
        let resp = request.send().await.unwrap();
        let code = resp.status();
        if !resp.status().is_success() && code != StatusCode::FORBIDDEN {
            eprintln!("fatal: LFS lock failed. Status: {}, Message: {}", code, resp.text().await.unwrap());
        }
        code
    }

    pub async fn unlock(&self, id: String, refspec: String, force: bool, basic_auth: Option<BasicAuth>) -> StatusCode {
        let url = self.lfs_url.join(&format!("locks/{}/unlock", id)).unwrap();
        let mut request = self.client.post(url).json(&UnlockRequest {
            force: Some(force),
            refs: Ref { name: refspec },
        });
        if let Some(auth) = basic_auth.clone() {
            request = request.basic_auth(auth.username, Some(auth.password));
        }
        let resp = request.send().await.unwrap();
        let code = resp.status();
        if !resp.status().is_success() && code != StatusCode::FORBIDDEN {
            eprintln!("fatal: LFS unlock failed. Status: {}, Message: {}", code, resp.text().await.unwrap());
        }
        code
    }

    /// List Locks for Verification
    pub async fn verify_locks(&self, query: VerifiableLockRequest, basic_auth: Option<BasicAuth>)
        -> (StatusCode, VerifiableLockList)
    {
        let url = self.lfs_url.join("locks/verify").unwrap();
        let mut request = self.client.post(url).json(&query);
        if let Some(auth) = basic_auth {
            request = request.basic_auth(auth.username, Some(auth.password));
        }
        let resp = request.send().await.unwrap();
        let code = resp.status();
        // By default, an LFS server that doesn't implement any locking endpoints should return 404.
        // This response will not halt any Git pushes.
        if !code.is_success() && code != StatusCode::NOT_FOUND && code != StatusCode::FORBIDDEN {
            eprintln!("fatal: LFS verify locks failed. Status: {}, Message: {}", code, resp.text().await.unwrap());
            return (code, VerifiableLockList {
                ours: Vec::new(),
                theirs: Vec::new(),
                next_cursor: String::default(),
            });
        }
        (code, resp.json::<VerifiableLockList>().await.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_request_vars() {
        let vars = RequestVars {
            oid: "123".to_string(),
            size: 123,
            ..Default::default()
        };
        println!("{:?}", serde_json::to_string(&vars).unwrap());
    }

    #[tokio::test]
    async fn test_github_batch() {
        let batch_request = BatchRequest {
            operation: "download".to_string(),
            transfers: vec![lfs::LFS_TRANSFER_API.to_string()],
            objects: vec![RequestVars {
                oid: "01cb1483670f1c497412f25f9f8f7dde31a8fab0960291035af03939ae1dfa6b".to_string(),
                size: 104103,
                ..Default::default()
            }],
            hash_algo: lfs::LFS_HASH_ALGO.to_string(),
        };
        let lfs_client = LFSClient::from_url(&Url::parse("https://github.com/web3infra-foundation/mega.git").unwrap());
        let request = lfs_client.client
            .post(lfs_client.batch_url.clone())
            .json(&batch_request)
            .headers(lfs::LFS_HEADERS.clone());
        println!("Request {:?}", request);
        let response = request.send().await.unwrap();
        let text = response.text().await.unwrap();
        println!("Text {:?}", text);
        let _resp = serde_json::from_str::<LfsBatchResponse>(&text).unwrap();
    }
}