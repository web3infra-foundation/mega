//!
//!
//!
use std::cmp::min;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::http::{Response, StatusCode};
use bytes::BytesMut;
use chrono::{prelude::*, Duration};
use common::errors::GitLFSError;
use entity::{locks, meta};
use futures::StreamExt;
use hyper::Request;
use rand::prelude::*;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use storage::driver::database::storage::ObjectStorage;
use storage::driver::file_storage::local_storage::MetaObject;

use crate::lfs::lfs_structs::{
    BatchResponse, BatchVars, LockList, LockRequest, LockResponse, ObjectError, UnlockRequest,
    UnlockResponse, VerifiableLockList, VerifiableLockRequest,
};

use super::lfs_structs::{Link, Lock, LockListQuery, Representation, RequestVars};
use super::LfsConfig;

pub async fn lfs_retrieve_lock(
    config: &LfsConfig,
    lock_list_query: LockListQuery,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("retrieving locks: {:?}", lock_list_query);
    let repo = lock_list_query
        .refspec
        .as_ref()
        .unwrap_or(&"".to_string())
        .to_string();
    let path = match lock_list_query.path.as_ref() {
        Some(val) => val.to_owned(),
        None => "".to_owned(),
    };
    let cursor = match lock_list_query.path.as_ref() {
        Some(val) => val.to_owned(),
        None => "".to_owned(),
    };
    let limit = match lock_list_query.path.as_ref() {
        Some(val) => val.to_owned(),
        None => "".to_owned(),
    };
    let mut resp = Response::builder();
    resp = resp.header("Content-Type", "application/vnd.git-lfs+json");

    let (locks, next_cursor, ok) =
        match lfs_get_filtered_locks(config.storage.clone(), &repo, &path, &cursor, &limit).await {
            Ok((locks, next)) => (locks, next, true),
            Err(_) => (vec![], "".to_string(), false),
        };

    let mut lock_list = LockList {
        locks: vec![],
        next_cursor: "".to_string(),
    };

    if !ok {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Lookup operation failed!".to_string(),
        ));
    } else {
        lock_list.locks = locks;
        lock_list.next_cursor = next_cursor;
    }

    let locks_response = serde_json::to_string(&lock_list).unwrap();
    println!("{:?}", locks_response);
    let body = Body::from(locks_response);

    Ok(resp.body(body).unwrap())
}

pub async fn lfs_verify_lock(
    config: &LfsConfig,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("req: {:?}", req);
    let mut resp = Response::builder();
    resp = resp.header("Content-Type", "application/vnd.git-lfs+json");

    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }

    let verifiable_lock_request: VerifiableLockRequest =
        serde_json::from_slice(request_body.freeze().as_ref()).unwrap();
    let mut limit = verifiable_lock_request.limit.unwrap_or(0);
    if limit == 0 {
        limit = 100;
    }

    let res = lfs_get_filtered_locks(
        config.storage.clone(),
        &verifiable_lock_request.refs.name,
        "",
        &verifiable_lock_request
            .cursor
            .unwrap_or("".to_string())
            .to_string(),
        &limit.to_string(),
    )
    .await;

    let (locks, next_cursor, ok) = match res {
        Ok((locks, next)) => (locks, next, true),
        Err(_) => (vec![], "".to_string(), false),
    };

    let mut lock_list = VerifiableLockList {
        ours: vec![],
        theirs: vec![],
        next_cursor: "".to_string(),
    };
    tracing::info!("acquired: {:?}", lock_list);

    if !ok {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Lookup operation failed!".to_string(),
        ));
    } else {
        lock_list.next_cursor = next_cursor;

        for lock in locks.iter() {
            if Option::is_none(&lock.owner) {
                lock_list.ours.push(lock.clone());
            } else {
                lock_list.theirs.push(lock.clone());
            }
        }
    }
    let locks_response = serde_json::to_string(&lock_list).unwrap();
    tracing::info!("sending: {:?}", locks_response);
    let body = Body::from(locks_response);

    Ok(resp.body(body).unwrap())
}

pub async fn lfs_create_lock(
    config: &LfsConfig,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("req: {:?}", req);
    let mut resp = Response::builder();
    resp = resp.header("Content-Type", "application/vnd.git-lfs+json");

    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }

    let lock_request: LockRequest = serde_json::from_slice(request_body.freeze().as_ref()).unwrap();
    println!("{:?}", lock_request);
    tracing::info!("acquired: {:?}", lock_request);
    let res = lfs_get_filtered_locks(
        config.storage.clone(),
        &lock_request.refs.name,
        &lock_request.path.to_string(),
        "",
        "1",
    )
    .await;

    let (locks, _, ok) = match res {
        Ok((locks, next)) => (locks, next, true),
        Err(_) => (vec![], "".to_string(), false),
    };

    if !ok {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed when filtering locks!".to_string(),
        ));
    }

    if !locks.is_empty() {
        return Err((StatusCode::CONFLICT, "Lock already exist".to_string()));
    }

    let lock = Lock {
        id: {
            let mut random_num = String::new();
            let mut rng = rand::thread_rng();
            for _ in 0..8 {
                random_num += &(rng.gen_range(0..9)).to_string();
            }
            random_num
        },
        path: lock_request.path.to_owned(),
        owner: None,
        locked_at: {
            let locked_at: DateTime<Utc> = Utc::now();
            locked_at.to_rfc3339()
        },
    };

    let ok = lfs_add_lock(
        config.storage.clone(),
        &lock_request.refs.name,
        vec![lock.clone()],
    )
    .await
    .is_ok();
    if !ok {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed when adding locks!".to_string(),
        ));
    }

    resp = resp.status(StatusCode::CREATED);

    let lock_response = LockResponse {
        lock,
        message: "".to_string(),
    };
    let lock_response = serde_json::to_string(&lock_response).unwrap();
    let body = Body::from(lock_response);

    Ok(resp.body(body).unwrap())
}

pub async fn lfs_delete_lock(
    config: &LfsConfig,
    id: &str,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    // Retrieve information from request body.
    tracing::info!("req: {:?}", req);
    let mut resp = Response::builder();
    resp = resp.header("Content-Type", "application/vnd.git-lfs+json");

    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }

    if id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Invalid lock id!".to_string()));
    }

    if request_body.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Deserialize operation failed!".to_string(),
        ));
    }
    let unlock_request: UnlockRequest =
        serde_json::from_slice(request_body.freeze().as_ref()).unwrap();

    let res = delete_lock(
        config.storage.clone(),
        &unlock_request.refs.name,
        None,
        id,
        unlock_request.force.unwrap_or(false),
    )
    .await;

    let (deleted_lock, ok) = match res {
        Ok(lock) => (lock, true),
        Err(_) => (
            Lock {
                id: "".to_string(),
                path: "".to_string(),
                owner: None,
                locked_at: { DateTime::<Utc>::MIN_UTC.to_rfc3339() },
            },
            false,
        ),
    };

    if !ok {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Delete operation failed!".to_string(),
        ));
    }

    if deleted_lock.id.is_empty()
        && deleted_lock.path.is_empty()
        && deleted_lock.owner.is_none()
        && deleted_lock.locked_at == DateTime::<Utc>::MIN_UTC.to_rfc3339()
    {
        return Err((StatusCode::NOT_FOUND, "Unable to find lock!".to_string()));
    }

    let unlock_response = UnlockResponse {
        lock: deleted_lock,
        message: "".to_string(),
    };
    tracing::info!("sending: {:?}", unlock_response);
    let unlock_response = serde_json::to_string(&unlock_response).unwrap();

    let body = Body::from(unlock_response);
    Ok(resp.body(body).unwrap())
}

pub async fn lfs_process_batch(
    config: &LfsConfig,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    // Extract the body to `BatchVars`.
    tracing::info!("req: {:?}", req);

    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }

    let mut batch_vars: BatchVars = serde_json::from_slice(request_body.freeze().as_ref()).unwrap();

    let bvo = &mut batch_vars.objects;
    for request in bvo {
        request.authorization = "".to_string();
    }
    tracing::info!("acquired: {:?}", batch_vars);

    let mut response_objects = Vec::<Representation>::new();

    // let db = Arc::new(state.storage.clone());
    // let config = Arc::new(state.config.clone());

    let server_url = format!("http://{}:{}", config.host, config.port);

    for object in batch_vars.objects {
        let meta = lfs_get_meta(config.storage.clone(), &object).await;

        // Found
        let found = meta.is_ok();
        let mut meta = meta.unwrap_or_default();
        if found && config.fs_storage.exist(&meta.oid) {
            response_objects.push(represent(&object, &meta, true, false, false, &server_url).await);
            continue;
        }

        // Not found
        if batch_vars.operation == "upload" {
            meta = lfs_put_meta(config.storage.clone(), &object).await.unwrap();
            response_objects.push(represent(&object, &meta, false, true, false, &server_url).await);
        } else {
            let rep = Representation {
                oid: object.oid.to_owned(),
                size: object.size,
                authenticated: None,
                actions: None,
                error: Some(ObjectError {
                    code: 404,
                    message: "Not found".to_owned(),
                }),
            };
            response_objects.push(rep);
        }
    }

    let batch_response = BatchResponse {
        transfer: "basic".to_string(),
        objects: response_objects,
        hash_algo: "sha256".to_string(),
    };

    let json = serde_json::to_string(&batch_response).unwrap();
    //DEBUG

    let mut resp = Response::builder();
    resp = resp.status(200);
    resp = resp.header("Content-Type", "application/vnd.git-lfs+json");

    let body = Body::from(json);
    let resp = resp.body(body).unwrap();
    println!("Sending: {:?}", resp);

    Ok(resp)
}

pub async fn lfs_upload_object(
    config: &LfsConfig,
    oid: &str,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("req: {:?}", req);
    // Load request parameters into struct.
    let request_vars = RequestVars {
        oid: oid.to_string(),
        authorization: "".to_string(),
        ..Default::default()
    };

    let meta = lfs_get_meta(config.storage.clone(), &request_vars)
        .await
        .unwrap();

    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }

    let res = config
        .fs_storage
        .put(&meta.oid, meta.size, &request_body.freeze())
        .await;
    if res.is_err() {
        lfs_delete_meta(config.storage.clone(), &request_vars)
            .await
            .unwrap();
        return Err((
            StatusCode::NOT_ACCEPTABLE,
            String::from("Header not acceptable!"),
        ));
    }
    let mut resp = Response::builder();
    resp = resp.header("Content-Type", "application/vnd.git-lfs");
    let resp = resp.body(Body::empty()).unwrap();

    Ok(resp)
}

pub async fn lfs_download_object(
    config: &LfsConfig,
    oid: &str,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("start downloading LFS object");

    // Load request parameters into struct.
    let request_vars = RequestVars {
        oid: oid.to_owned(),
        authorization: "".to_owned(),
        ..Default::default()
    };

    let meta = lfs_get_meta(config.storage.clone(), &request_vars)
        .await
        .unwrap();

    let bytes = config.fs_storage.get(&meta.oid).await.unwrap();
    let mut resp = Response::builder();
    resp = resp.status(200);
    let body = Body::from(bytes);
    Ok(resp.body(body).unwrap())
}

pub async fn represent(
    rv: &RequestVars,
    meta: &MetaObject,
    download: bool,
    upload: bool,
    use_tus: bool,
    server_url: &str,
) -> Representation {
    let mut rep = Representation {
        oid: meta.oid.to_owned(),
        size: meta.size,
        authenticated: Some(true),
        actions: None,
        error: None,
    };

    let mut header: HashMap<String, String> = HashMap::new();
    let mut verify_header: HashMap<String, String> = HashMap::new();

    header.insert("Accept".to_string(), "application/vnd.git-lfs".to_owned());

    if !rv.authorization.is_empty() {
        header.insert("Authorization".to_string(), rv.authorization.to_owned());
        verify_header.insert("Authorization".to_string(), rv.authorization.to_owned());
    }

    if download {
        let mut actions = HashMap::new();
        actions.insert(
            "download".to_string(),
            Link {
                href: { rv.download_link(server_url.to_string()).await },
                header: header.clone(),
                expires_at: {
                    let expire_time: DateTime<Utc> = Utc::now() + Duration::seconds(86400);
                    expire_time.to_rfc3339()
                },
            },
        );
        rep.actions = Some(actions);
    }

    if upload {
        let mut actions = HashMap::new();
        actions.insert(
            "upload".to_string(),
            Link {
                href: { rv.upload_link(server_url.to_string()).await },
                header: header.clone(),
                expires_at: {
                    let expire_time: DateTime<Utc> = Utc::now() + Duration::seconds(86400);
                    expire_time.to_rfc3339()
                },
            },
        );
        rep.actions = Some(actions);
        if use_tus {
            let mut actions = HashMap::new();
            actions.insert(
                "verify".to_string(),
                Link {
                    href: { rv.verify_link(server_url.to_string()).await },
                    header: verify_header.clone(),
                    expires_at: {
                        let expire_time: DateTime<Utc> = Utc::now() + Duration::seconds(86400);
                        expire_time.to_rfc3339()
                    },
                },
            );
            rep.actions = Some(actions);
        }
    }

    rep
}

async fn lfs_get_filtered_locks(
    storage: Arc<dyn ObjectStorage>,
    refspec: &str,
    path: &str,
    cursor: &str,
    limit: &str,
) -> Result<(Vec<Lock>, String), GitLFSError> {
    let mut locks = match lfs_get_locks(storage, refspec).await {
        Ok(locks) => locks,
        Err(_) => vec![],
    };

    println!("Locks retrieved: {:?}", locks);

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
            next = locks[size as usize].id.to_owned();
        }
        let _ = locks.split_off(size as usize);
    }

    Ok((locks, next))
}

async fn lfs_get_locks(
    storage: Arc<dyn ObjectStorage>,
    refspec: &str,
) -> Result<Vec<Lock>, GitLFSError> {
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
    storage: Arc<dyn ObjectStorage>,
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

            let mut lock_to: locks::ActiveModel = val.into();
            lock_to.data = Set(d.to_owned());
            let res = lock_to.update(storage.get_connection()).await;
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
            let lock_to = locks::ActiveModel {
                id: Set(repo.to_owned()),
                data: Set(data.to_owned()),
            };
            let res = locks::Entity::insert(lock_to)
                .exec(storage.get_connection())
                .await;
            match res.is_ok() {
                true => Ok(()),
                false => Err(GitLFSError::GeneralError("".to_string())),
            }
        }
    }
}

async fn lfs_get_meta(
    storage: Arc<dyn ObjectStorage>,
    v: &RequestVars,
) -> Result<MetaObject, GitLFSError> {
    let result = storage.get_meta_by_id(v.oid.clone()).await.unwrap();

    match result {
        Some(val) => Ok(MetaObject {
            oid: val.oid,
            size: val.size,
            exist: val.exist,
        }),
        None => Err(GitLFSError::GeneralError("".to_string())),
    }
}

async fn lfs_put_meta(
    storage: Arc<dyn ObjectStorage>,
    v: &RequestVars,
) -> Result<MetaObject, GitLFSError> {
    // Check if already exist.
    let result = storage.get_meta_by_id(v.oid.clone()).await.unwrap();
    if let Some(result) = result {
        return Ok(MetaObject {
            oid: result.oid,
            size: result.size,
            exist: true,
        });
    }

    // Put into database if not exist.
    let meta = MetaObject {
        oid: v.oid.to_string(),
        size: v.size,
        exist: true,
    };

    let meta_to = meta::ActiveModel {
        oid: Set(meta.oid.to_owned()),
        size: Set(meta.size.to_owned()),
        exist: Set(true),
    };

    let res = meta::Entity::insert(meta_to)
        .exec(storage.get_connection())
        .await;
    match res {
        Ok(_) => Ok(meta),
        Err(err) => Err(GitLFSError::GeneralError(err.to_string())),
    }
}

async fn lfs_delete_meta(
    storage: Arc<dyn ObjectStorage>,
    v: &RequestVars,
) -> Result<(), GitLFSError> {
    let res = storage.delete_meta_by_id(v.oid.to_owned()).await;
    match res {
        Ok(_) => Ok(()),
        Err(_) => Err(GitLFSError::GeneralError("".to_string())),
    }
}

async fn delete_lock(
    storage: Arc<dyn ObjectStorage>,
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
                    lock_to_delete.id = lock.id.to_owned();
                    lock_to_delete.path = lock.path.to_owned();
                    lock_to_delete.owner = lock.owner.clone();
                    lock_to_delete.locked_at = lock.locked_at.to_owned();
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

            let mut lock_to: locks::ActiveModel = val.into();
            lock_to.data = Set(data.to_owned());
            let res = lock_to.update(storage.get_connection()).await;
            match res.is_ok() {
                true => Ok(lock_to_delete),
                false => Err(GitLFSError::GeneralError("".to_string())),
            }
        }
        // Not exist, error.
        None => Err(GitLFSError::GeneralError("".to_string())),
    }
}
