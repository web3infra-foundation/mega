//mod controller;
pub mod ext;
pub mod lfs;
pub mod route;
mod utils;

use std::collections::HashSet;

use crate::util::config;
use ceres::lfs::lfs_structs::{BatchRequest, Operation, Ref, RequestObject, VerifiableLockRequest};
use libra::internal::protocol::{
    https_client::BasicAuth,
    lfs_client::{LFSClient, LfsBatchResponse},
    ProtocolClient,
};
use libra::utils::lfs as lfsutils;
use mercury::internal::{object::types::ObjectType, pack::entry::Entry};
use reqwest::StatusCode;

#[allow(unused)]
trait ScorpioLFS {
    async fn scorpio_push<'a, I>(&self, objs: I) -> Result<(), ()>
    where
        I: IntoIterator<Item = &'a Entry>;

    fn scorpio_new(mono_path: &str) -> Self;
}

impl ScorpioLFS for LFSClient {
    fn scorpio_new(mono_path: &str) -> Self {
        let url = format!("{}/{}", config::lfs_url(), mono_path);
        let url = url::Url::parse(&url).unwrap();
        LFSClient::from_url(&url)
    }

    async fn scorpio_push<'a, I>(&self, objs: I) -> Result<(), ()>
    where
        I: IntoIterator<Item = &'a Entry>,
    {
        // filter pointer file within blobs
        let mut lfs_oids = Vec::new();
        for blob in objs.into_iter().filter(|e| e.obj_type == ObjectType::Blob) {
            let oid = lfsutils::parse_pointer_data(&blob.data);
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
                        name: utils::current_refspec().unwrap(),
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
            transfers: vec![lfsutils::LFS_TRANSFER_API.to_string()],
            objects: lfs_objs,
            hash_algo: lfsutils::LFS_HASH_ALGO.to_string(),
        };

        let response = BasicAuth::send(|| async {
            self.client
                .post(self.batch_url.clone())
                .json(&batch_request)
                .headers(lfsutils::LFS_HEADERS.clone())
        })
        .await
        .unwrap();

        let resp = response.json::<LfsBatchResponse>().await.map_err(|e| {
            eprintln!("fatal: LFS batch request failed. Error: {}", e);
        })?;
        println!(
            "LFS push response:\n {:#?}",
            serde_json::to_value(&resp).unwrap()
        );

        // TODO: parallel upload
        for obj in resp.objects {
            let file_path = lfs::lfs_object_path(&obj.oid);
            println!("{:?}", serde_json::to_string(&obj).unwrap());
            self.upload_object(obj, &file_path).await?;
        }
        println!("LFS objects push completed.");
        Ok(())
    }
}
