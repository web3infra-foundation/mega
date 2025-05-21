//! This module contains handlers for handling requests related to Git LFS (Large File Storage).
//!
//! The handlers in this module are responsible for handling various Git LFS operations such as
//! retrieving locks, verifying locks, creating and deleting locks, processing batch requests,
//! downloading and uploading objects, etc.
//!
//! Each handler corresponds to a specific endpoint or operation in the Git LFS protocol.
//! Error handling is done to return appropriate responses in case of success or failure.
//! These handlers are used in an Axum-based web application to handle Git LFS requests.
//!
//! # References
//!
//! - Git LFS Documentation: [https://git-lfs.github.com/](https://git-lfs.github.com/)
//! - Axum Documentation: [https://docs.rs/axum/](https://docs.rs/axum/)
//!
//! # Note
//!
//! Add more specific details and examples as needed to describe each handler's functionality.
//!
//! # Examples and Usage
//!
//! - `lfs_retrieve_lock`: Handles retrieving locks for Git LFS objects.
//! - `lfs_verify_lock`: Handles verifying locks for Git LFS objects.
//! - `lfs_create_lock`: Handles creating locks for Git LFS objects.
//! - `lfs_delete_lock`: Handles deleting locks for Git LFS objects.
//! - `lfs_process_batch`: Handles batch processing requests for Git LFS objects.
//! - `lfs_download_object`: Handles downloading Git LFS objects.
//! - `lfs_upload_object`: Handles uploading Git LFS objects.
//!
//! # Errors
//!
//! The handlers return `Result<Response, (StatusCode, String)>` to handle success or error cases.
//! Error responses are constructed with appropriate status codes and error messages.
//!
//! # Panics
//!
//! The code in these handlers is designed to handle errors gracefully and avoid panics.
//! However, certain unexpected situations might lead to panics, which should be minimized.
//!
//! # Security Considerations
//!
//! Ensure proper authentication and authorization mechanisms are implemented
//! when using these handlers in a web application to prevent unauthorized access.
use std::collections::HashMap;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    response::Response,
    routing::{get, post, put},
    Json,
};
use futures::TryStreamExt;
use utoipa_axum::router::OpenApiRouter;

use ceres::lfs::{
    handler,
    lfs_structs::{
        BatchRequest, FetchchunkResponse, LockList, LockListQuery, LockRequest, LockResponse,
        RequestObject, UnlockRequest, UnlockResponse, VerifiableLockRequest,
    },
};
use common::errors::GitLFSError;

use crate::api::MonoApiServiceState;

const LFS_CONTENT_TYPE: &str = "application/vnd.git-lfs+json";

/// The [LFS Server Discovery](https://github.com/git-lfs/git-lfs/blob/main/docs/api/server-discovery.md)
/// document describes the server LFS discovery protocol.
pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new()
        .route("/objects/{object_id}", get(lfs_download_object))
        .route("/objects/{object_id}/{chunk_id}", get(lfs_download_chunk))
        .route("/objects/{object_id}", put(lfs_upload_object))
        .route("/locks", get(list_locks))
        .route("/locks", post(create_lock))
        .route("/locks/verify", post(list_locks_for_verification))
        .route("/locks/{id}/unlock", post(delete_lock))
        .route("/objects/batch", post(lfs_process_batch))
        .route("/objects/{object_id}/chunks", get(lfs_fetch_chunk_ids))
}

pub async fn list_locks(
    state: State<MonoApiServiceState>,
    Query(query): Query<LockListQuery>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let result: Result<LockList, GitLFSError> =
        handler::lfs_retrieve_lock(state.context.lfs_stg(), query).await;
    match result {
        Ok(lock_list) => {
            let body = serde_json::to_string(&lock_list).unwrap_or_default();
            Ok(Response::builder()
                .header("Content-Type", LFS_CONTENT_TYPE)
                .body(Body::from(body))
                .unwrap())
        }
        Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, err.to_string())),
    }
}

pub async fn list_locks_for_verification(
    state: State<MonoApiServiceState>,
    Json(json): Json<VerifiableLockRequest>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let result = handler::lfs_verify_lock(state.context.lfs_stg(), json).await;
    match result {
        Ok(lock_list) => {
            let body = serde_json::to_string(&lock_list).unwrap_or_default();
            Ok(Response::builder()
                .header("Content-Type", LFS_CONTENT_TYPE)
                .body(Body::from(body))
                .unwrap())
        }
        Err(err) => Ok({
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Error: {}", err)))
                .unwrap()
        }),
    }
}

pub async fn create_lock(
    state: State<MonoApiServiceState>,
    Json(json): Json<LockRequest>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let result = handler::lfs_create_lock(state.context.lfs_stg(), json).await;
    match result {
        Ok(lock) => {
            let lock_response = LockResponse {
                lock,
                message: "".to_string(),
            };
            let body = serde_json::to_string(&lock_response).unwrap_or_default();
            Ok(Response::builder()
                .header("Content-Type", LFS_CONTENT_TYPE)
                .status(StatusCode::CREATED)
                .body(Body::from(body))
                .unwrap())
        }
        Err(err) => Ok({
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Error: {}", err)))
                .unwrap()
        }),
    }
}

pub async fn delete_lock(
    state: State<MonoApiServiceState>,
    Path(id): Path<String>,
    Json(json): Json<UnlockRequest>,
) -> Result<Response, (StatusCode, String)> {
    let result = handler::lfs_delete_lock(state.context.lfs_stg(), &id, json).await;

    match result {
        Ok(lock) => {
            let unlock_response = UnlockResponse {
                lock,
                message: "".to_string(),
            };
            let body = serde_json::to_string(&unlock_response).unwrap_or_default();
            Ok(Response::builder()
                .header("Content-Type", LFS_CONTENT_TYPE)
                .body(Body::from(body))
                .unwrap())
        }
        Err(err) => Ok({
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Error: {}", err)))
                .unwrap()
        }),
    }
}

pub async fn lfs_process_batch(
    state: State<MonoApiServiceState>,
    Json(json): Json<BatchRequest>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let result = handler::lfs_process_batch(&state.context, json, &state.listen_addr).await;

    match result {
        Ok(res) => {
            let body = serde_json::to_string(&res).unwrap_or_default();
            Ok(Response::builder()
                .header("Content-Type", LFS_CONTENT_TYPE)
                .body(Body::from(body))
                .unwrap())
        }
        Err(err) => Ok({
            tracing::error!("Error: {}", err);

            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Error: {}", err)))
                .unwrap()
        }),
    }
}

pub async fn lfs_fetch_chunk_ids(
    state: State<MonoApiServiceState>,
    Path(oid): Path<String>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let result = handler::lfs_fetch_chunk_ids(&state.context, &oid, &state.listen_addr).await;
    match result {
        Ok(response) => {
            let size = response.iter().fold(0, |acc, chunk| acc + chunk.size);
            let fetch_response = FetchchunkResponse {
                oid,
                size,
                chunks: response,
            };
            let body = serde_json::to_string(&fetch_response).unwrap_or_default();
            Ok(Response::builder()
                .header("Content-Type", LFS_CONTENT_TYPE)
                .body(Body::from(body))
                .unwrap())
        }
        Err(err) => Ok({
            tracing::error!("Error: {}", err);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Error: {}", err)))
                .unwrap()
        }),
    }
}

pub async fn lfs_download_object(
    state: State<MonoApiServiceState>,
    Path(oid): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let result = handler::lfs_download_object(state.context.clone(), oid.clone()).await;
    match result {
        Ok(byte_stream) => Ok(Response::builder()
            .header("Content-Type", LFS_CONTENT_TYPE)
            .body(Body::from_stream(byte_stream))
            .unwrap()),
        Err(err) => Ok({
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Error: {}", err)))
                .unwrap()
        }),
    }
}

pub async fn lfs_download_chunk(
    state: State<MonoApiServiceState>,
    Path((origin_object_id, chunk_id)): Path<(String, String)>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Response, (StatusCode, String)> {
    let offset = query_params
        .get("offset")
        .and_then(|offset| offset.parse::<u64>().ok());
    let size = query_params
        .get("size")
        .and_then(|size| size.parse::<u64>().ok());
    if offset.is_none() || size.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Valid offset and size query parameters are required".to_string(),
        ));
    }
    let result = handler::lfs_download_chunk(
        state.context.clone(),
        &origin_object_id,
        &chunk_id,
        offset.unwrap(),
        size.unwrap(),
    )
    .await;
    match result {
        Ok(bytes) => Ok(Response::builder()
            .header("Content-Type", LFS_CONTENT_TYPE)
            .body(Body::from(bytes))
            .unwrap()),
        Err(err) => Ok({
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Error: {}", err)))
                .unwrap()
        }),
    }
}

pub async fn lfs_upload_object(
    state: State<MonoApiServiceState>,
    Path(oid): Path<String>,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let req_obj = RequestObject {
        oid,
        ..Default::default()
    };

    // Collect bytes asynchronously from the stream into a Vec<u8>
    let body_bytes: Vec<u8> = req
        .into_body()
        .into_data_stream()
        .try_fold(Vec::new(), |mut acc, chunk| async move {
            acc.extend_from_slice(&chunk);
            Ok(acc)
        })
        .await
        .unwrap();

    let result = handler::lfs_upload_object(&state.context, &req_obj, body_bytes).await;
    match result {
        Ok(_) => Ok(Response::builder()
            .header("Content-Type", LFS_CONTENT_TYPE)
            .body(Body::empty())
            .unwrap()),
        Err(err) => Ok({
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Error: {}", err)))
                .unwrap()
        }),
    }
}
