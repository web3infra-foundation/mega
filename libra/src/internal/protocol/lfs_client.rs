use crate::command;
use crate::internal::config::Config;
use crate::internal::protocol::https_client::BasicAuth;
use crate::internal::protocol::ProtocolClient;
use crate::utils::{lfs, util};
use anyhow::anyhow;
use ceres::lfs::lfs_structs::{
    Action, BatchRequest, ChunkDownloadObject, FetchchunkResponse, LockList, LockListQuery,
    LockRequest, ObjectError, Operation, Ref, RequestObject, ResponseObject, UnlockRequest,
    VerifiableLockList, VerifiableLockRequest,
};
use futures_util::StreamExt;
use mercury::internal::object::types::ObjectType;
use mercury::internal::pack::entry::Entry;
use reqwest::{Client, StatusCode};
use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::OnceCell;
use url::Url;

#[derive(Debug)]
pub struct LFSClient {
    pub batch_url: Url,
    pub lfs_url: Url,
    pub client: Client,
    pub bootstrap: Option<(String, u16)>, // for p2p: (bootstrap_node, ztm_agent_port)
}
static LFS_CLIENT: OnceCell<LFSClient> = OnceCell::const_new();
impl LFSClient {
    /// Get LFSClient instance
    /// - DO NOT use `async_static!`: No IDE Code Completion & lagging
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
            bootstrap: None,
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

    // TODO add one method that both support Server & P2P
    /// Only for p2p
    pub fn from_bootstrap_node(bootstrap_node: &str, ztm_agent_port: u16) -> Self {
        let client = Client::builder()
            .default_headers(lfs::LFS_HEADERS.clone())
            .build()
            .unwrap();
        Self {
            batch_url: Url::parse("https://invalid.com").unwrap(),
            lfs_url: Url::parse("https://invalid.com").unwrap(),
            client,
            bootstrap: Some((bootstrap_node.to_string(), ztm_agent_port)),
        }
    }

    /// push LFS objects to remote server
    pub async fn push_objects<'a, I>(&self, objs: I) -> Result<(), ()>
    where
        I: IntoIterator<Item = &'a Entry>,
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
            lfs_objs.push(RequestObject {
                oid: oid.to_owned(),
                size,
                ..Default::default()
            })
        }

        if lfs_objs.is_empty() {
            tracing::info!("No LFS objects to push.");
            return Ok(());
        }

        {
            // verify locks
            let (code, locks) = self
                .verify_locks(VerifiableLockRequest {
                    refs: Ref {
                        name: command::lfs::current_refspec().await.unwrap(),
                    },
                    ..Default::default()
                })
                .await;

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
                let ours = locks
                    .ours
                    .iter()
                    .filter(|l| {
                        let oid = lfs::get_oid_by_path(&l.path);
                        oids.contains(&oid)
                    })
                    .collect::<Vec<_>>();
                if !ours.is_empty() {
                    println!("The following files are locked by you, consider unlocking them:");
                    for lock in ours {
                        println!("  - {}", lock.path);
                    }
                }
                let theirs = locks
                    .theirs
                    .iter()
                    .filter(|l| {
                        let oid = lfs::get_oid_by_path(&l.path);
                        oids.contains(&oid)
                    })
                    .collect::<Vec<_>>();
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
            operation: Operation::Upload,
            transfers: vec![lfs::LFS_TRANSFER_API.to_string()],
            objects: lfs_objs,
            hash_algo: lfs::LFS_HASH_ALGO.to_string(),
        };

        let response = BasicAuth::send(|| async {
            self.client
                .post(self.batch_url.clone())
                .json(&batch_request)
                .headers(lfs::LFS_HEADERS.clone())
        })
        .await
        .unwrap();

        let resp = response.json::<LfsBatchResponse>().await.unwrap();
        tracing::debug!(
            "LFS push response:\n {:#?}",
            serde_json::to_value(&resp).unwrap()
        );

        // TODO: parallel upload
        for obj in resp.objects {
            let file_path = lfs::lfs_object_path(&obj.oid);
            self.upload_object(obj, &file_path).await?;
        }
        println!("LFS objects push completed.");
        Ok(())
    }

    /// push LFS object to remote server, didn't need local lfs storage
    pub async fn push_object(&self, oid: &str, file: &Path) -> Result<(), ()> {
        let batch_request = BatchRequest {
            operation: Operation::Upload,
            transfers: vec![lfs::LFS_TRANSFER_API.to_string()],
            objects: vec![RequestObject {
                oid: oid.to_owned(),
                size: file.metadata().unwrap().len() as i64,
                ..Default::default()
            }],
            hash_algo: lfs::LFS_HASH_ALGO.to_string(),
        };

        let response = BasicAuth::send(|| async {
            self.client
                .post(self.batch_url.clone())
                .json(&batch_request)
                .headers(lfs::LFS_HEADERS.clone())
        })
        .await
        .unwrap();

        let resp = response.json::<LfsBatchResponse>().await.unwrap();
        tracing::debug!(
            "LFS push response:\n {:#?}",
            serde_json::to_value(&resp).unwrap()
        );
        assert_eq!(
            resp.objects.len(),
            1,
            "fatal: LFS push failed. No object found."
        );

        // self.upload_object(resp.objects).await?;
        let obj = resp.objects.into_iter().next().unwrap();
        self.upload_object(obj, file).await?;
        println!("LFS objects push completed.");
        Ok(())
    }

    /// upload (PUT) one LFS file to remote server
    pub async fn upload_object(&self, object: ResponseObject, file: &Path) -> Result<(), ()> {
        if let Some(err) = object.error {
            eprintln!(
                "fatal: LFS upload failed. Code: {}, Message: {}",
                err.code, err.message
            );
            return Err(());
        }

        if let Some(actions) = object.actions {
            let upload_link = actions.get(&Action::Upload);
            if upload_link.is_none() {
                eprintln!("fatal: LFS upload failed. No upload action found");
                return Err(());
            }

            println!("Uploading LFS file: {}", object.oid);
            let link = upload_link.unwrap();

            let resp = BasicAuth::send(|| async {
                let mut request = self.client.put(&link.href);
                for (k, v) in &link.header {
                    request = request.header(k, v);
                }

                let content = tokio::fs::File::open(file).await.unwrap();
                let progress_bar =
                    util::default_progress_bar(content.metadata().await.unwrap().len());

                let stream = tokio_util::io::ReaderStream::new(content);
                let progress_stream = stream.map(move |chunk| {
                    if let Ok(ref data) = chunk {
                        progress_bar.inc(data.len() as u64);
                    }
                    chunk
                });
                request.body(reqwest::Body::wrap_stream(progress_stream))
            })
            .await
            .unwrap();

            if !resp.status().is_success() {
                eprintln!(
                    "fatal: LFS upload failed. Status: {}, Message: {}",
                    resp.status(),
                    resp.text().await.unwrap()
                );
                return Err(());
            }
            println!("Uploaded.");
        } else {
            tracing::debug!("LFS file {} already exists on remote server", object.oid);
        }
        Ok(())
    }

    /// Just for resume download
    async fn update_file_checksum(file: &mut tokio::fs::File, checksum: &mut Context) {
        file.seek(tokio::io::SeekFrom::Start(0)).await.unwrap();
        let mut buf = [0u8; 8192];
        loop {
            let n = file.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            checksum.update(&buf[..n]);
        }
    }

    #[allow(clippy::type_complexity)]
    /// download (GET) one LFS file from remote server
    pub async fn download_object(
        &self,
        oid: &str,
        size: u64,
        path: impl AsRef<Path>,
        mut reporter: Option<(
            &mut (dyn FnMut(f64) -> anyhow::Result<()> + Send), // progress callback
            f64,                                                // step
        )>,
    ) -> anyhow::Result<()> {
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

        let response = BasicAuth::send(|| async {
            self.client
                .post(self.batch_url.clone())
                .json(&batch_request)
                .headers(lfs::LFS_HEADERS.clone())
        })
        .await?;

        let text = response.text().await?;
        tracing::debug!(
            "LFS download response:\n {:#?}",
            serde_json::from_str::<serde_json::Value>(&text)?
        );
        let resp = serde_json::from_str::<LfsBatchResponse>(&text)?;
        let obj = resp.objects.first().expect("No object"); // Only get first
        if obj.error.is_some() || obj.actions.is_none() {
            let unknown_err = ObjectError {
                code: 0,
                message: "Unknown error".to_string(),
            };
            let err = obj.error.as_ref().unwrap_or(&unknown_err);
            eprintln!(
                "fatal: LFS download failed (BatchRequest). Code: {}, Message: {}",
                err.code, err.message
            );
            return Err(anyhow!("LFS download failed."));
        }

        let link = obj
            .actions
            .as_ref()
            .unwrap()
            .get(&Action::Download)
            .unwrap();

        let mut is_chunked = false;
        // Chunk API
        let chunk_size; // infer that all chunks are the same size!
        let links = match self.fetch_chunks(&link.href).await {
            Ok(chunks) => {
                is_chunked = true;
                chunk_size = chunks.first().map(|c| c.size);
                tracing::info!("LFS Chunk API supported.");
                chunks.into_iter().map(|c| c.link).collect()
            }
            Err(_) => {
                chunk_size = Some(size as i64);
                vec![link.clone()]
            }
        };

        let mut checksum = Context::new(&SHA256);
        let mut got_parts = 0;
        let mut file = if links.len() <= 1 || lfs::parse_pointer_file(&path).is_ok() {
            // pointer file or Not Chunks, truncate
            tokio::fs::File::create(path).await?
        } else {
            // for Chunks, calc offset to resume download
            let mut file = tokio::fs::File::options()
                .write(true)
                .read(true)
                .create(true)
                .truncate(false)
                .open(&path)
                .await?;
            let file_len = file.metadata().await?.len();
            if file_len > size {
                println!("Local file size is larger than remote, truncate to 0.");
                file.set_len(0).await?; // clear
                file.seek(tokio::io::SeekFrom::Start(0)).await?;
            } else if file_len > 0 {
                let chunk_size = chunk_size.unwrap() as u64;
                got_parts = file_len / chunk_size;
                let file_offset = got_parts * chunk_size;
                println!(
                    "Resume download from offset: {}, part: {}",
                    file_offset,
                    got_parts + 1
                );
                file.set_len(file_offset).await?; // truncate
                Self::update_file_checksum(&mut file, &mut checksum).await; // resume checksum
                file.seek(tokio::io::SeekFrom::End(0)).await?;
            }
            file
        };

        println!("Downloading LFS file: {}", oid);
        let parts = links.len();
        let mut downloaded: u64 = file.metadata().await?.len();
        let mut last_progress = 0.0;
        let start_part = got_parts as usize;
        for link in links.iter().skip(start_part) {
            got_parts += 1;
            if is_chunked {
                println!("- part: {}/{}", got_parts, parts);
            }

            let response = BasicAuth::send(|| async {
                let mut request = self.client.get(&link.href);
                for (k, v) in &link.header {
                    request = request.header(k, v);
                }
                request
            })
            .await?;
            if !response.status().is_success() {
                eprintln!(
                    "fatal: LFS download failed. Status: {}, Message: {}",
                    response.status(),
                    response.text().await?
                );
                return Err(anyhow!("LFS download failed."));
            }

            let cur_chunk_size = if (got_parts as usize) < parts {
                chunk_size.unwrap() as u64
            } else {
                // last part
                size - (parts as u64 - 1) * chunk_size.unwrap() as u64
            };
            let pb = util::default_progress_bar(cur_chunk_size);
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                // TODO: progress bar TODO: multi-thread or async
                let chunk = chunk?;
                file.write_all(&chunk).await?;
                checksum.update(&chunk);

                // report progress
                if let Some((ref mut report_fn, step)) = reporter {
                    downloaded += chunk.len() as u64;
                    let progress = (downloaded as f64 / size as f64) * 100.0;
                    if progress >= last_progress + step {
                        last_progress = progress;
                        report_fn(progress)?;
                    }
                } else {
                    // mutually exclusive with reporter
                    pb.inc(chunk.len() as u64);
                }
            }
            pb.finish_and_clear();
        }
        let checksum = hex::encode(checksum.finish().as_ref());
        if checksum == oid {
            println!("Downloaded.");
            Ok(())
        } else {
            eprintln!("fatal: LFS download failed. Checksum mismatch: {} != {}. Fallback to pointer file.", checksum, oid);
            let pointer = lfs::format_pointer_string(oid, size);
            file.set_len(0).await?; // clear
            file.seek(tokio::io::SeekFrom::Start(0)).await?; // ensure
            file.write_all(pointer.as_bytes()).await?;
            Err(anyhow!("Checksum mismatch, fallback to pointer file."))
        }
    }

    /// Only for MonoRepo (mega)
    async fn fetch_chunks(&self, obj_link: &str) -> Result<Vec<ChunkDownloadObject>, ()> {
        let mut url = Url::parse(obj_link).unwrap();
        let path = url.path().trim_end_matches('/');
        url.set_path(&(path.to_owned() + "/chunks")); // reserve query params (for GitHub link)

        let resp = BasicAuth::send(|| async { self.client.get(url.clone()) })
            .await
            .unwrap();
        let code = resp.status();
        if code == StatusCode::NOT_FOUND || code == StatusCode::FORBIDDEN {
            // GitHub maybe return 403
            tracing::info!("Remote LFS Server not support Chunks API, or forbidden.");
            return Err(());
        } else if !code.is_success() {
            tracing::debug!(
                "fatal: LFS get chunk hrefs failed. Status: {}, Message: {}",
                code,
                resp.text().await.unwrap()
            );
            return Err(());
        }
        let mut res = resp.json::<FetchchunkResponse>().await.unwrap();
        // sort by offset
        res.chunks.sort_by(|a, b| a.offset.cmp(&b.offset));
        Ok(res.chunks)
    }
}

#[cfg(feature = "p2p")]
impl LFSClient {
    /// download (GET) one LFS file peer-to-peer
    #[allow(clippy::type_complexity)]
    pub async fn download_object_p2p(
        &self,
        file_uri: &str, // p2p protocol
        path: impl AsRef<Path>,
        mut reporter: Option<(
            &mut (dyn FnMut(f64) -> anyhow::Result<()> + Send), // progress callback
            f64,                                                // step
        )>,
    ) -> anyhow::Result<()> {
        let (bootstrap_node, ztm_agent_port) = match &self.bootstrap {
            Some(value) => value,
            None => return Err(anyhow!("fatal: No bootstrap node set for P2P download.")),
        };

        let hash = gemini::lfs::get_file_hash_from_origin(file_uri.to_owned()).unwrap();
        tracing::info!("Downloading LFS file: {}", hash);
        let peer_ports = gemini::lfs::create_lfs_download_tunnel(
            bootstrap_node.clone(),
            *ztm_agent_port,
            file_uri.to_owned(),
        )
        .await
        .unwrap();
        if peer_ports.is_empty() {
            eprintln!("fatal: No peer online, download failed");
            return Err(anyhow!("fatal: No peer online."));
        }
        tracing::debug!("P2P download tunnel ports: {:?}", peer_ports);

        let lfs_info =
            match gemini::lfs::get_lfs_chunks_info(bootstrap_node.clone(), hash.clone()).await {
                // auth?
                Some(chunks) => chunks,
                None => return Err(anyhow!("fatal: LFS Chunk API failed.")),
            };
        let mut chunks = lfs_info.chunks;
        if chunks.is_empty() {
            eprintln!("fatal: LFS Chunk API failed. No chunks found.");
            return Err(anyhow!("fatal: No chunks found."));
        }
        chunks.sort_by(|a, b| a.offset.cmp(&b.offset));
        tracing::debug!("LFS chunks: {:?}", chunks.len());

        // infer that all chunks share same size! (except last one)
        let chunk_size = chunks.first().unwrap().size as usize;
        let mut checksum = Context::new(&SHA256);
        if let Some(parent) = path.as_ref().parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = tokio::fs::File::create(path).await?;
        for (i, chunk) in chunks.iter().enumerate() {
            // TODO parallel download
            println!("- part: {}/{}", i + 1, chunks.len());
            let mut retry = 0;
            let data = loop {
                // retry
                let mut downloaded = i * chunk_size; // TODO support resume
                let mut last_progress = downloaded as f64 / lfs_info.size as f64 * 100.0;
                let pb = util::default_progress_bar(chunk.size as u64);
                let url = format!(
                    "http://localhost:{}/objects/{}/{}",
                    peer_ports[(i + retry) % peer_ports.len()],
                    hash,
                    chunk.sub_oid
                );
                let data = self
                    .download_chunk(
                        &url,
                        &chunk.sub_oid,
                        chunk.size as usize,
                        chunk.offset as usize,
                        |size| {
                            if let Some((ref mut report_fn, step)) = reporter {
                                downloaded += size;
                                let progress = (downloaded as f64 / lfs_info.size as f64) * 100.0;
                                if progress >= last_progress + step {
                                    last_progress = progress;
                                    report_fn(progress).unwrap();
                                }
                            } else {
                                pb.inc(size as u64);
                            }
                        },
                    )
                    .await;
                pb.finish_and_clear();
                match data {
                    Ok(data) => break data,
                    Err(e) => {
                        eprintln!("fatal: LFS download failed. Error: {}. Retry", e);
                        retry += 1;
                        if retry > 5 {
                            eprintln!("fatal: LFS download failed. Retry limit exceeded.");
                            return Err(anyhow!("LFS download failed."));
                        }
                    }
                }
            };
            checksum.update(&data);
            file.write_all(&data).await?;
        }
        let checksum = hex::encode(checksum.finish().as_ref());
        if checksum == hash {
            println!("Downloaded(p2p).");
            Ok(())
        } else {
            eprintln!("fatal: LFS download failed. Checksum mismatch: {} != {}. Fallback to pointer file.", checksum, hash);
            file.set_len(0).await?; // clear
            file.rewind().await?; // == seek(0)
            let pointer = lfs::format_pointer_string(&hash, lfs_info.size as u64);
            file.write_all(pointer.as_bytes()).await?;
            Err(anyhow!("Checksum mismatch, fallback to pointer file."))
        }
    }

    pub async fn download_chunk(
        &self,
        url: &str,
        hash: &str,
        size: usize,
        offset: usize,
        mut callback: impl FnMut(usize),
    ) -> anyhow::Result<Vec<u8>> {
        let response = BasicAuth::send(|| async {
            self.client
                .get(url)
                .query(&[("offset", offset), ("size", size)])
        })
        .await?;
        if !response.status().is_success() {
            eprintln!(
                "fatal: LFS download failed. Status: {}, Message: {}",
                response.status(),
                response.text().await?
            );
            return Err(anyhow!("LFS download failed."));
        }
        let mut buffer = Vec::with_capacity(size);
        let mut checksum = Context::new(&SHA256);
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.write_all(&chunk).await?;
            checksum.update(&chunk);

            // report progress
            callback(chunk.len());
        }
        let checksum = hex::encode(checksum.finish().as_ref());
        if checksum != hash {
            eprintln!(
                "fatal: chunk download failed. Chunk checksum mismatch: {} != {}",
                checksum, hash
            );
            return Err(anyhow!("Chunk checksum mismatch."));
        }
        Ok(buffer)
    }
}

// LFS locks API
impl LFSClient {
    pub async fn get_locks(&self, query: LockListQuery) -> LockList {
        let url = self.lfs_url.join("locks").unwrap();
        let query = [
            ("id", query.id),
            ("path", query.path),
            ("limit", query.limit),
            ("cursor", query.cursor),
            ("refspec", query.refspec),
        ];
        let response = BasicAuth::send(|| async { self.client.get(url.clone()).query(&query) })
            .await
            .unwrap();
        if !response.status().is_success() {
            eprintln!(
                "fatal: LFS get locks failed. Status: {}, Message: {}",
                response.status(),
                response.text().await.unwrap()
            );
            return LockList {
                locks: Vec::new(),
                next_cursor: String::default(),
            };
        }

        response.json::<LockList>().await.unwrap()
    }

    /// lock an LFS file
    /// - `refspec` is must in Mega Server, but optional in Git Doc
    pub async fn lock(&self, path: String, refspec: String) -> StatusCode {
        let url = self.lfs_url.join("locks").unwrap();
        let resp = BasicAuth::send(|| async {
            self.client.post(url.clone()).json(&LockRequest {
                path: path.clone(),
                refs: Ref {
                    name: refspec.clone(),
                },
            })
        })
        .await
        .unwrap();
        let code = resp.status();
        if !resp.status().is_success() && code != StatusCode::FORBIDDEN {
            eprintln!(
                "fatal: LFS lock failed. Status: {}, Message: {}",
                code,
                resp.text().await.unwrap()
            );
        }
        code
    }

    pub async fn unlock(&self, id: String, refspec: String, force: bool) -> StatusCode {
        let url = self.lfs_url.join(&format!("locks/{}/unlock", id)).unwrap();
        let resp = BasicAuth::send(|| async {
            self.client.post(url.clone()).json(&UnlockRequest {
                force: Some(force),
                refs: Ref {
                    name: refspec.clone(),
                },
            })
        })
        .await
        .unwrap();
        let code = resp.status();
        if !resp.status().is_success() && code != StatusCode::FORBIDDEN {
            eprintln!(
                "fatal: LFS unlock failed. Status: {}, Message: {}",
                code,
                resp.text().await.unwrap()
            );
        }
        code
    }

    /// List Locks for Verification
    pub async fn verify_locks(
        &self,
        query: VerifiableLockRequest,
    ) -> (StatusCode, VerifiableLockList) {
        let url = self.lfs_url.join("locks/verify").unwrap();
        let resp = BasicAuth::send(|| async { self.client.post(url.clone()).json(&query) })
            .await
            .unwrap();
        let code = resp.status();
        // By default, an LFS server that doesn't implement any locking endpoints should return 404.
        // This response will not halt any Git pushes.
        if !code.is_success() && code != StatusCode::NOT_FOUND && code != StatusCode::FORBIDDEN {
            eprintln!(
                "fatal: LFS verify locks failed. Status: {}, Message: {}",
                code,
                resp.text().await.unwrap()
            );
            return (
                code,
                VerifiableLockList {
                    ours: Vec::new(),
                    theirs: Vec::new(),
                    next_cursor: String::default(),
                },
            );
        }
        (code, resp.json::<VerifiableLockList>().await.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use crate::utils;

    use super::*;
    #[test]
    fn test_request_vars() {
        let vars = RequestObject {
            oid: "123".to_string(),
            size: 123,
            ..Default::default()
        };
        println!("{:?}", serde_json::to_string(&vars).unwrap());
    }

    #[tokio::test]
    async fn test_github_batch() {
        let batch_request = BatchRequest {
            operation: Operation::Download,
            transfers: vec![lfs::LFS_TRANSFER_API.to_string()],
            objects: vec![RequestObject {
                oid: "01cb1483670f1c497412f25f9f8f7dde31a8fab0960291035af03939ae1dfa6b".to_string(),
                size: 104103,
                ..Default::default()
            }],
            hash_algo: lfs::LFS_HASH_ALGO.to_string(),
        };
        let lfs_client = LFSClient::from_url(
            &Url::parse("https://github.com/web3infra-foundation/mega.git").unwrap(),
        );
        let request = lfs_client
            .client
            .post(lfs_client.batch_url.clone())
            .json(&batch_request)
            .headers(lfs::LFS_HEADERS.clone());
        println!("Request {:?}", request);
        let response = request.send().await.unwrap();
        let text = response.text().await.unwrap();
        println!("Text {:?}", text);
        let _resp = serde_json::from_str::<LfsBatchResponse>(&text).unwrap();
    }

    #[tokio::test]
    #[ignore] // need to start local mega server
    async fn test_push_object() {
        let file_map = mercury::test_utils::setup_lfs_file().await;
        let file = file_map
            .get("git-2d187177923cd618a75da6c6db45bb89d92bd504.pack")
            .unwrap();
        let client = LFSClient::from_url(&Url::parse("http://localhost:8000").unwrap());
        let oid = utils::lfs::calc_lfs_file_hash(file).unwrap();

        match client.push_object(&oid, file).await {
            Ok(_) => println!("Pushed successfully."),
            Err(err) => eprintln!("Push failed: {:?}", err),
        }
    }

    #[tokio::test]
    #[cfg(feature = "p2p")]
    #[ignore] // need to start local mega server
    async fn test_download_chunk() {
        let file_map = mercury::test_utils::setup_lfs_file().await;
        let file = file_map
            .get("git-2d187177923cd618a75da6c6db45bb89d92bd504.pack")
            .unwrap();
        let client = LFSClient::from_url(&Url::parse("http://localhost:8000").unwrap());
        let oid = utils::lfs::calc_lfs_file_hash(file).unwrap();
        let sub_oid =
            "ee225720cc31599c749fbe9b18f6c8346fa3246839f0dea7ffd3224dbb067952".to_string(); // offset 83886080 size 20971520
        let url = format!("http://localhost:8000/objects/{}/{}", oid, sub_oid);
        let size = 20971520;
        let offset = 83886080;
        let data = client
            .download_chunk(&url, &sub_oid, size, offset, |_| {})
            .await
            .unwrap();
        assert_eq!(data.len(), size);
    }
}
