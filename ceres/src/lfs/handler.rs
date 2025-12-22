use std::cmp::min;

use anyhow::Result;
use bytes::Bytes;
use chrono::prelude::*;
use futures::Stream;
use rand::prelude::*;
use tokio_stream::wrappers::ReceiverStream;

use callisto::lfs_locks;
use common::config::PackConfig;
use common::errors::{GitLFSError, MegaError};
use jupiter::storage::Storage;
use jupiter::storage::lfs_db_storage::LfsDbStorage;

use crate::lfs::lfs_structs::{
    BatchRequest, BatchResponse, ChunkDownloadObject, Link, Lock, LockList, LockListQuery,
    LockRequest, MetaObject, ObjectError, Operation, RequestObject, ResCondition, ResponseObject,
    TransferMode, UnlockRequest, VerifiableLockList, VerifiableLockRequest,
};

pub async fn lfs_retrieve_lock(
    storage: LfsDbStorage,
    query: LockListQuery,
) -> Result<LockList, GitLFSError> {
    let mut lock_list = LockList {
        locks: vec![],
        next_cursor: "".to_string(),
    };
    match lfs_get_filtered_locks(
        storage,
        &query.refspec,
        &query.path,
        &query.cursor,
        &query.limit,
    )
    .await
    {
        Ok((locks, next)) => {
            lock_list.locks = locks;
            lock_list.next_cursor = next;
            Ok(lock_list)
        }
        Err(_) => Err(GitLFSError::GeneralError(
            "Lookup operation failed!".to_string(),
        )),
    }
}

pub async fn lfs_verify_lock(
    storage: LfsDbStorage,
    req: VerifiableLockRequest,
) -> Result<VerifiableLockList, MegaError> {
    let mut limit = req.limit.unwrap_or(0);
    if limit == 0 {
        limit = 100;
    }
    let res = lfs_get_filtered_locks(
        storage,
        &req.refs.name,
        "",
        &req.cursor.clone().unwrap_or("".to_string()).to_string(),
        &limit.to_string(),
    )
    .await;

    let mut lock_list = VerifiableLockList {
        ours: vec![],
        theirs: vec![],
        next_cursor: "".to_string(),
    };
    match res {
        Ok((locks, next_cursor)) => {
            lock_list.next_cursor = next_cursor;

            for lock in locks.iter() {
                if Option::is_none(&lock.owner) {
                    lock_list.ours.push(lock.clone());
                } else {
                    lock_list.theirs.push(lock.clone());
                }
            }
        }
        Err(_) => return Err(MegaError::Other("Lookup operation failed!".to_string())),
    };
    Ok(lock_list)
}

pub async fn lfs_create_lock(storage: LfsDbStorage, req: LockRequest) -> Result<Lock, GitLFSError> {
    let res = lfs_get_filtered_locks(
        storage.clone(),
        &req.refs.name,
        &req.path.to_string(),
        "",
        "1",
    )
    .await;

    match res {
        Ok((locks, _)) => {
            if !locks.is_empty() {
                return Err(GitLFSError::GeneralError("Lock already exist".to_string()));
            }
        }
        Err(_) => {
            return Err(GitLFSError::GeneralError(
                "Failed when filtering locks!".to_string(),
            ));
        }
    };

    let lock = Lock {
        id: {
            let mut random_num = String::new();
            let mut rng = rand::rng();
            for _ in 0..8 {
                random_num += &(rng.random_range(0..9)).to_string();
            }
            random_num
        },
        path: req.path.to_owned(),
        owner: None,
        locked_at: {
            let locked_at: DateTime<Utc> = Utc::now();
            locked_at.to_rfc3339()
        },
    };

    match lfs_add_lock(storage.clone(), &req.refs.name, vec![lock.clone()]).await {
        Ok(_) => Ok(lock),
        Err(_) => Err(GitLFSError::GeneralError(
            "Failed when adding locks!".to_string(),
        )),
    }
}

pub async fn lfs_delete_lock(
    storage: LfsDbStorage,
    id: &str,
    unlock_request: UnlockRequest,
) -> Result<Lock, GitLFSError> {
    if id.is_empty() {
        return Err(GitLFSError::GeneralError("Invalid lock id!".to_string()));
    }
    let res = delete_lock(
        storage,
        &unlock_request.refs.name,
        None,
        id,
        unlock_request.force.unwrap_or(false),
    )
    .await;
    match res {
        Ok(deleted_lock) => {
            if deleted_lock.id.is_empty()
                && deleted_lock.path.is_empty()
                && deleted_lock.owner.is_none()
                && deleted_lock.locked_at == DateTime::<Utc>::MIN_UTC.to_rfc3339()
            {
                Err(GitLFSError::GeneralError(
                    "Unable to find lock!".to_string(),
                ))
            } else {
                Ok(deleted_lock)
            }
        }
        Err(_) => Err(GitLFSError::GeneralError(
            "Delete operation failed!".to_string(),
        )),
    }
}

///
///
/// Reference:
///     1. [Git LFS Batch API](https://github.com/git-lfs/git-lfs/blob/main/docs/api/batch.md)
pub async fn lfs_process_batch(
    storage: &Storage,
    request: BatchRequest,
    listen_addr: &str,
) -> Result<BatchResponse, GitLFSError> {
    let objects = request.objects;

    let mut response_objects = Vec::new();
    let file_storage = storage.lfs_file_storage();
    let db_storage = storage.lfs_db_storage();
    let config = storage.config().lfs.clone();
    for object in objects {
        let meta_res = lfs_get_meta(db_storage.clone(), &object.oid).await.unwrap();
        let meta = match meta_res {
            Some(meta) => meta,
            None => {
                if request.operation == Operation::Upload {
                    // Save to database if not exist.
                    let meta = MetaObject::new(&object, &config);
                    db_storage
                        .new_lfs_object(meta.clone().into())
                        .await
                        .unwrap();
                    meta
                } else {
                    response_objects.push(ResponseObject::failed_with_err(
                        &object,
                        ObjectError {
                            code: 404,
                            message: "Not found".to_owned(),
                        },
                    ));
                    continue;
                }
            }
        };
        let enable_split = meta.splited && config.local.enable_split;
        let file_exist = file_storage.exist_object(&meta.oid, enable_split).await;
        let download_url = file_storage
            .download_url(&meta.oid, listen_addr)
            .await
            .unwrap();
        let upload_url = file_storage
            .upload_url(&meta.oid, listen_addr)
            .await
            .unwrap();

        response_objects.push(ResponseObject::new(
            &meta,
            ResCondition {
                file_exist,
                operation: request.operation.clone(),
                use_tus: false,
            },
            &download_url,
            &upload_url,
        ));
    }

    Ok(BatchResponse {
        transfer: TransferMode::BASIC,
        objects: response_objects,
        hash_algo: "sha256".to_string(),
    })
}

/// if server enable split, then return a list of chunk ids.
/// else return an error.
pub async fn lfs_fetch_chunk_ids(
    storage: &Storage,
    oid: &str,
    listen_addr: &str,
) -> Result<Vec<ChunkDownloadObject>, GitLFSError> {
    let config = storage.config().lfs.clone();

    if !config.local.enable_split {
        return Err(GitLFSError::GeneralError(
            "Server didn't run in `split` mode, didn't support chunk ids".to_string(),
        ));
    }
    let db_storage = storage.lfs_db_storage();

    let meta = lfs_get_meta(db_storage.clone(), oid)
        .await
        .map_err(|_| GitLFSError::GeneralError("".to_string()))?
        .unwrap();
    assert!(meta.splited, "database didn't match the split mode");

    let relations = db_storage
        .get_lfs_relations(oid)
        .await
        .map_err(|_| GitLFSError::GeneralError("".to_string()))?;

    if relations.is_empty() {
        return Err(GitLFSError::GeneralError(
            "oid didn't have chunks".to_string(),
        ));
    }
    let mut response_objects = Vec::<ChunkDownloadObject>::new();

    for relation in relations {
        // Reuse RequestArgs to create a link
        let req_obj = RequestObject {
            oid: relation.sub_oid.clone(),
            size: relation.size,
            ..Default::default()
        };
        let download_url = storage
            .lfs_file_storage()
            .download_url(&req_obj.oid, listen_addr)
            .await
            .unwrap();
        response_objects.push(ChunkDownloadObject {
            sub_oid: relation.sub_oid,
            size: relation.size,
            offset: relation.offset,
            link: Link::new(&download_url),
        });
    }
    Ok(response_objects)
}

/// Upload object to storage.
/// if server enable split, split the object and upload each part to storage, save the relationship to database.
pub async fn lfs_upload_object(
    storage: &Storage,
    req_obj: &RequestObject,
    body_bytes: Vec<u8>,
) -> Result<(), GitLFSError> {
    let config = storage.config().lfs.clone();
    let db_storage = storage.lfs_db_storage();
    let file_storage = storage.lfs_file_storage();

    let meta = if let Some(meta) = lfs_get_meta(db_storage.clone(), &req_obj.oid).await? {
        tracing::debug!("upload lfs object {} size: {}", meta.oid, meta.size);
        meta
    } else {
        return Err(GitLFSError::GeneralError(String::from("Not found ")));
    };
    let split_size = match PackConfig::get_size_from_str(&config.local.split_size, || Ok(0)) {
        Ok(split_size) => split_size,
        Err(err) => return Err(GitLFSError::GeneralError(err)),
    };

    if config.local.enable_split && meta.splited {
        // assert!(request_vars.size == body_bytes.len() as i64, "size didn't match: {} != {}", request_vars.size, body_bytes.len()); // TODO: git client, request_vars.size is `0`!!
        // split object to blocks
        match file_storage
            .put_object_with_chunk(&meta.oid, &body_bytes, split_size)
            .await
        {
            Ok(_) => {}
            Err(_) => {
                lfs_delete_meta(&db_storage, req_obj).await?;
                // TODO: whether/how to delete the uploaded blocks.
                return Err(GitLFSError::GeneralError(String::from(
                    "Header not acceptable!",
                )));
            }
        };
    } else {
        // normal mode
        let res = file_storage.put_object(&meta.oid, body_bytes).await;
        if res.is_err() {
            lfs_delete_meta(&db_storage, req_obj).await.unwrap();
            return Err(GitLFSError::GeneralError(String::from(
                "Header not acceptable!",
            )));
        }
    }
    Ok(())
}

/// Download object from storage.
/// when server enable split,  if OID is a complete object, then splice the object and return it.
pub async fn lfs_download_object(
    storage: Storage,
    oid: String,
) -> Result<impl Stream<Item = Result<Bytes, GitLFSError>>, GitLFSError> {
    let db_storage = storage.lfs_db_storage();
    let file_storage = storage.lfs_file_storage();

    let meta = lfs_get_meta(db_storage.clone(), &oid).await?;
    match meta {
        Some(meta) => {
            if meta.splited {
                // client didn't support split, splice the object and return it.
                let relations = db_storage.get_lfs_relations(&meta.oid).await.unwrap();
                if relations.is_empty() {
                    return Err(GitLFSError::GeneralError(
                        "oid didn't have chunks".to_string(),
                    ));
                }
                let (tx, rx) = tokio::sync::mpsc::channel(1);
                let oid = oid.clone();
                tokio::spawn(async move {
                    let chunks_len = relations.len();
                    for relation in relations {
                        let sub_bytes = file_storage.get_object(&relation.sub_oid).await.unwrap();
                        if let Err(err) = tx.send(Ok(sub_bytes)).await {
                            tracing::error!(
                                "lfs object download failed, failed to send chunk [{}], error: {}",
                                relation.offset,
                                err
                            );
                            break;
                        }
                    }
                    tracing::debug!(
                        "lfs object download completed for oid: {}, {} chunks",
                        oid,
                        chunks_len
                    );
                });
                Ok(ReceiverStream::new(rx))
            } else {
                let meta = lfs_get_meta(db_storage, &oid).await?.unwrap();
                let bytes = file_storage.get_object(&meta.oid).await.unwrap();
                let (tx, rx) = tokio::sync::mpsc::channel(1);
                tx.send(Ok(bytes)).await.unwrap();
                Ok(ReceiverStream::new(rx))
            }
        }
        None => {
            // check if the oid is a part of a split object, if so, return the part.
            if !lfs_check_sub_oid_exist(db_storage, &oid).await.unwrap() {
                return Err(GitLFSError::GeneralError(
                    "oid didn't belong to any object".to_string(),
                ));
            }

            let bytes = file_storage.get_object(&oid).await.unwrap();
            // because return type must be `ReceiverStream`, so we need to wrap the bytes into a stream.
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            tx.send(Ok(bytes)).await.unwrap();
            Ok(ReceiverStream::new(rx))
        }
    }
}

/// Download a chunk from a large object.
/// It's used when server didn't have splited chunk, but client request a chunk.
/// If the server enable split, then the chunk must be a splited chunk, rather than a random part of the object.
pub async fn lfs_download_chunk(
    storage: Storage,
    origin_oid: &str,
    chunk_oid: &String,
    offset: u64,
    size: u64,
) -> Result<Bytes, GitLFSError> {
    let config = &storage.config().lfs;
    let db_storage = storage.lfs_db_storage();
    let file_storage = storage.lfs_file_storage();

    // check if the chunk is already exist.
    if config.local.enable_split {
        let relations = db_storage.get_lfs_relations(origin_oid).await.unwrap();
        let chunk_relation = relations.iter().find(|r| &r.sub_oid == chunk_oid);
        if chunk_relation.is_none() {
            return Err(GitLFSError::GeneralError(
                "Chunk not found in split object".to_string(),
            ));
        }
        let chunk = file_storage.get_object(chunk_oid).await.unwrap();
        Ok(chunk)
    } else {
        // return part of the original object.
        let bytes = file_storage.get_object(origin_oid).await.unwrap();
        let chunk_bytes = bytes[offset as usize..(offset + size) as usize].to_vec();
        // check hash
        let chunk_hash = hex::encode(ring::digest::digest(&ring::digest::SHA256, &chunk_bytes));
        if chunk_hash != *chunk_oid {
            return Err(GitLFSError::GeneralError(
                "Chunk hash not match".to_string(),
            ));
        }
        Ok(Bytes::from(chunk_bytes))
    }
}

async fn lfs_get_filtered_locks(
    storage: LfsDbStorage,
    refspec: &str,
    path: &str,
    cursor: &str,
    limit: &str,
) -> Result<(Vec<Lock>, String), GitLFSError> {
    let mut locks = (lfs_get_locks(storage, refspec).await).unwrap_or_default();

    tracing::debug!("Locks retrieved: {:?}", locks);

    if !cursor.is_empty() {
        let mut last_seen = -1;
        for (i, v) in locks.iter().enumerate() {
            if v.id == *cursor {
                last_seen = i as i32;
                break;
            }
        }

        if last_seen > -1 {
            locks = locks.split_off(last_seen as usize);
        } else {
            // Cursor not found.
            return Err(GitLFSError::GeneralError("".to_string()));
        }
    }

    if !path.is_empty() {
        let mut filterd = Vec::<Lock>::new();
        for lock in locks.iter() {
            if lock.path == *path {
                filterd.push(Lock {
                    id: lock.id.to_owned(),
                    path: lock.path.to_owned(),
                    owner: lock.owner.clone(),
                    locked_at: lock.locked_at.to_owned(),
                });
            }
        }
        locks = filterd;
    }

    let mut next = "".to_string();
    if !limit.is_empty() {
        let mut size = limit.parse::<i64>().unwrap();
        size = min(size, locks.len() as i64);

        if size + 1 < locks.len() as i64 {
            locks[size as usize].id.clone_into(&mut next);
        }
        let _ = locks.split_off(size as usize);
    }

    Ok((locks, next))
}

async fn lfs_get_locks(storage: LfsDbStorage, refspec: &str) -> Result<Vec<Lock>, GitLFSError> {
    let result = storage.get_lock_by_id(refspec).await.unwrap();
    match result {
        Some(val) => {
            let data = val.data;
            let locks: Vec<Lock> = serde_json::from_str(&data).unwrap();
            Ok(locks)
        }
        None => Err(GitLFSError::GeneralError("".to_string())),
    }
}

async fn lfs_add_lock(
    storage: LfsDbStorage,
    repo: &str,
    locks: Vec<Lock>,
) -> Result<(), GitLFSError> {
    let result = storage.get_lock_by_id(repo).await.unwrap();

    match result {
        // Update
        Some(val) => {
            let d = val.data.to_owned();
            let mut locks_from_data = if !d.is_empty() {
                let locks_from_data: Vec<Lock> = serde_json::from_str(&d).unwrap();
                locks_from_data
            } else {
                vec![]
            };
            let mut locks = locks;
            locks_from_data.append(&mut locks);

            locks_from_data.sort_by(|a, b| {
                a.locked_at
                    .partial_cmp(&b.locked_at)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let d = serde_json::to_string(&locks_from_data).unwrap();

            // must turn into `ActiveModel` before modify, or update failed.
            // let mut val = val.into_active_model();
            // val.data = Set(d);
            let res = storage.update_lock(val, &d).await;
            match res.is_ok() {
                true => Ok(()),
                false => Err(GitLFSError::GeneralError("".to_string())),
            }
        }
        // Insert
        None => {
            let mut locks = locks;
            locks.sort_by(|a, b| {
                a.locked_at
                    .partial_cmp(&b.locked_at)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let data = serde_json::to_string(&locks).unwrap();
            let lock_to = lfs_locks::Model {
                id: repo.to_owned(),
                data: data.to_owned(),
            };

            let res = storage.new_lock(lock_to).await;
            match res.is_ok() {
                true => Ok(()),
                false => Err(GitLFSError::GeneralError("".to_string())),
            }
        }
    }
}

async fn lfs_get_meta(storage: LfsDbStorage, oid: &str) -> Result<Option<MetaObject>, GitLFSError> {
    Ok(storage.get_lfs_object(oid).await.unwrap().map(|m| m.into()))
}

async fn lfs_delete_meta(
    storage: &LfsDbStorage,
    req_obj: &RequestObject,
) -> Result<(), GitLFSError> {
    let res = storage.delete_lfs_object(req_obj.oid.to_owned()).await;
    storage.delete_lfs_relations(&req_obj.oid).await.unwrap();
    match res {
        Ok(_) => Ok(()),
        Err(_) => Err(GitLFSError::GeneralError("".to_string())),
    }
}

async fn delete_lock(
    storage: LfsDbStorage,
    repo: &str,
    _user: Option<String>,
    id: &str,
    force: bool,
) -> Result<Lock, GitLFSError> {
    let result = storage.get_lock_by_id(repo).await.unwrap();
    match result {
        // Exist, then delete.
        Some(val) => {
            let d = val.data.to_owned();
            let locks_from_data = if !d.is_empty() {
                let locks_from_data: Vec<Lock> = serde_json::from_str(&d).unwrap();
                locks_from_data
            } else {
                vec![]
            };

            let mut new_locks = Vec::<Lock>::new();
            let mut lock_to_delete = Lock {
                id: "".to_owned(),
                path: "".to_owned(),
                owner: None,
                locked_at: {
                    let locked_at: DateTime<Utc> = DateTime::<Utc>::MIN_UTC;
                    locked_at.to_rfc3339()
                },
            };

            for lock in locks_from_data.iter() {
                if lock.id == *id {
                    if Option::is_some(&lock.owner) && !force {
                        return Err(GitLFSError::GeneralError("".to_string()));
                    }
                    lock.id.clone_into(&mut lock_to_delete.id);
                    lock.path.clone_into(&mut lock_to_delete.path);
                    lock_to_delete.owner.clone_from(&lock.owner);
                    lock.locked_at.clone_into(&mut lock_to_delete.locked_at);
                } else if !lock.id.is_empty() {
                    new_locks.push(Lock {
                        id: lock.id.to_owned(),
                        path: lock.path.to_owned(),
                        owner: lock.owner.clone(),
                        locked_at: lock.locked_at.to_owned(),
                    });
                }
            }
            if lock_to_delete.id.is_empty() {
                return Err(GitLFSError::GeneralError("".to_string()));
            }

            // No locks remains, delete the repo from database.
            if new_locks.is_empty() {
                storage.delete_lock_by_id(repo.to_owned()).await;
                return Ok(lock_to_delete);
            }

            // Update remaining locks.
            let data = serde_json::to_string(&new_locks).unwrap();
            let res = storage.update_lock(val, &data).await;
            match res.is_ok() {
                true => Ok(lock_to_delete),
                false => Err(GitLFSError::GeneralError("".to_string())),
            }
        }
        // Not exist, error.
        None => Err(GitLFSError::GeneralError("".to_string())),
    }
}

async fn lfs_check_sub_oid_exist(
    storage: LfsDbStorage,
    sub_oid: &str,
) -> Result<bool, GitLFSError> {
    let result = storage.get_lfs_relations_ori_oid(sub_oid).await.unwrap();
    Ok(!result.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lfs::lfs_structs::{Action, Ref, ResCondition, ResponseObject};

    #[test]
    fn response_object_download_existing() {
        let meta = MetaObject {
            oid: "oid1".into(),
            size: 10,
            exist: true,
            splited: false,
        };
        let res = ResponseObject::new(
            &meta,
            ResCondition {
                file_exist: true,
                operation: Operation::Download,
                use_tus: false,
            },
            "http://dl",
            "http://ul",
        );
        assert!(res.actions.is_some());
        let actions = res.actions.unwrap();
        assert!(actions.contains_key(&Action::Download));
        assert!(res.error.is_none());
    }

    #[test]
    fn response_object_upload_new() {
        let meta = MetaObject {
            oid: "oid2".into(),
            size: 20,
            exist: false,
            splited: false,
        };
        let res = ResponseObject::new(
            &meta,
            ResCondition {
                file_exist: false,
                operation: Operation::Upload,
                use_tus: false,
            },
            "http://dl",
            "http://ul",
        );
        let actions = res.actions.expect("upload should provide actions");
        assert!(actions.contains_key(&Action::Upload));
        assert!(res.error.is_none());
    }

    #[test]
    fn response_object_download_missing_sets_error() {
        let meta = MetaObject {
            oid: "oid3".into(),
            size: 30,
            exist: false,
            splited: false,
        };
        let res = ResponseObject::new(
            &meta,
            ResCondition {
                file_exist: false,
                operation: Operation::Download,
                use_tus: false,
            },
            "http://dl",
            "http://ul",
        );
        assert!(res.actions.is_none());
        assert!(res.error.is_some());
        assert_eq!(res.error.unwrap().code, 404);
    }

    #[test]
    fn unlock_request_defaults() {
        let req = UnlockRequest::default();
        assert!(req.force.is_none());
        assert_eq!(req.refs, Ref { name: "".into() });
    }
}
