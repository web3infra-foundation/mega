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
use axum::{
    body::Body,
    extract::{FromRequest, State},
    http::{Request, StatusCode},
    response::Response,
    Json,
};

use ceres::lfs::{
    handler,
    lfs_structs::{
        BatchResponse, LockList, LockListQuery, LockRequest, LockResponse, RequestVars,
        UnlockRequest, UnlockResponse, VerifiableLockRequest,
    },
    LfsConfig,
};
use common::{errors::GitLFSError, model::GetParams};
use futures::TryStreamExt;

use crate::https_server::AppState;

const LFS_CONTENT_TYPE: &str = "application/vnd.git-lfs+json";

pub async fn lfs_retrieve_lock(
    config: &LfsConfig,
    params: GetParams,
) -> Result<Response<Body>, (StatusCode, String)> {
    // Load query parameters into struct.
    let lock_list_query = LockListQuery {
        path: params.path.unwrap_or_default(),
        id: params.id.unwrap_or_default(),
        cursor: params.cursor.unwrap_or_default(),
        limit: params.limit.unwrap_or_default(),
        refspec: params.refspec.unwrap_or_default(),
    };

    let result: Result<LockList, GitLFSError> =
        handler::lfs_retrieve_lock(config, lock_list_query).await;
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

pub async fn lfs_verify_lock(
    state: State<AppState>,
    config: &LfsConfig,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("req: {:?}", req);

    let request = Json::from_request(req, &state)
        .await
        .unwrap_or_else(|_| Json(VerifiableLockRequest::default()));

    let result = handler::lfs_verify_lock(config, request.0).await;
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

pub async fn lfs_create_lock(
    state: State<AppState>,
    config: &LfsConfig,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let request = Json::from_request(req, &state)
        .await
        .unwrap_or_else(|_| Json(LockRequest::default()));

    let result = handler::lfs_create_lock(config, request.0).await;
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

pub async fn lfs_delete_lock(
    state: State<AppState>,
    config: &LfsConfig,
    path: &str,
    req: Request<Body>,
) -> Result<Response, (StatusCode, String)> {
    let tokens: Vec<&str> = path.split('/').collect();
    let id = tokens[tokens.len() - 2];
    let request = Json::from_request(req, &state)
        .await
        .unwrap_or_else(|_| Json(UnlockRequest::default()));

    let result = handler::lfs_delete_lock(config, id, request.0).await;

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
    state: State<AppState>,
    config: &LfsConfig,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let request = Json::from_request(req, &state).await.unwrap();
    let result = handler::lfs_process_batch(config, request.0).await;

    match result {
        Ok(response_objects) => {
            let batch_response = BatchResponse {
                transfer: "basic".to_string(),
                objects: response_objects,
                hash_algo: "sha256".to_string(),
            };
            let body = serde_json::to_string(&batch_response).unwrap_or_default();
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

pub async fn lfs_fetch_chunk_ids(
    state: State<AppState>,
    config: &LfsConfig,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let request = Json::from_request(req, &state).await.unwrap();
    let result = handler::lfs_fetch_chunk_ids(config, &request).await;
    match result {
        Ok(response) => {
            let body = serde_json::to_string(&response).unwrap_or_default();
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

pub async fn lfs_download_object(
    config: &LfsConfig,
    path: &str,
) -> Result<Response, (StatusCode, String)> {
    let tokens: Vec<&str> = path.split('/').collect();
    // Load request parameters into struct.
    let request_vars = RequestVars {
        oid: tokens[tokens.len() - 1].to_owned(),
        authorization: "".to_owned(),
        ..Default::default()
    };
    let result = handler::lfs_download_object(config, &request_vars).await;
    match result {
        Ok(bytes) => Ok(Response::builder().body(Body::from(bytes)).unwrap()),
        Err(err) => Ok({
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Error: {}", err)))
                .unwrap()
        }),
    }
}

pub async fn lfs_upload_object(
    config: &LfsConfig,
    path: &str,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let tokens: Vec<&str> = path.split('/').collect();
    // Load request parameters into struct.
    let request_vars = RequestVars {
        oid: tokens[tokens.len() - 1].to_string(),
        authorization: "".to_string(),
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

    let result = handler::lfs_upload_object(config, &request_vars, &body_bytes).await;
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
