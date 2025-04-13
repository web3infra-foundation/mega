// src/api/lfs/mod.rs
use axum::{
    extract::{Json, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use libra::internal::protocol::lfs_client::LFSClient;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ceres::lfs::lfs_structs::{LockListQuery, Lock};
use crate::util::{scorpio_config, GPath};

use super::{lfs, utils::{self, current_refspec}};

#[derive(Debug, Deserialize)]
struct TrackRequest {
    patterns: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct UntrackRequest {
    paths: Vec<String>,
}

#[derive(Debug, Serialize)]
struct TrackResponse {
    tracked_patterns: usize
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(self),
        ).into_response()
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/lfs/attributes", post(track).delete(untrack))
        .route("/lfs/locks", get(list_locks))
        .route("/lfs/locks/:path", post(create_lock).delete(remove_lock))
        //.route("/lfs/files", get(list_files))
}

// ==== Track/Untrack Endpoints ====
async fn track(Json(payload): Json<TrackRequest>) -> Result<Json<TrackResponse>, ErrorResponse> {
    let attr_path = utils::lfs_attribate();
    let converted_patterns = convert_patterns_to_workdir(payload.patterns);
    let pat_size = converted_patterns.len();
    lfs::add_lfs_patterns(attr_path.to_str().unwrap(), converted_patterns)
        .await
        .map_err(|e| ErrorResponse { error: e.to_string() })?;

    Ok(Json(TrackResponse {
        tracked_patterns: pat_size,
    }))
}

async fn untrack(Json(payload): Json<UntrackRequest>) -> Result<Json<TrackResponse>, ErrorResponse> {
    let attr_path = utils::lfs_attribate();
    let converted_paths = convert_patterns_to_workdir(payload.paths);

    let re = lfs::untrack_lfs_patterns(attr_path.to_str().unwrap(), converted_paths).await;
    match re {
        Ok(_) => Ok(Json(TrackResponse {
            tracked_patterns: 0,
        })),
        Err(_) => Err(ErrorResponse{
            error: "untrace error".to_owned(),
        }),
    }
}

// ==== Lock Management Endpoints ====
#[derive(Debug, Deserialize)]
struct ListLocksQuery {
    id: Option<String>,
    path: Option<String>,
    limit: Option<u64>,
}

#[derive(Debug, Serialize)]
struct LockResponse {
    locks: Vec<Lock>,
}

async fn list_locks(
    Query(query): Query<ListLocksQuery>
) -> Result<Json<LockResponse>, ErrorResponse> {
    let refspec = current_refspec()
        .ok_or_else(|| ErrorResponse { error: "Could not determine current ref".to_string() })?;

    let locks = LFSClient::get()
        .await
        .get_locks(LockListQuery {
            id: query.id.unwrap_or_default(),
            path: query.path.unwrap_or_default(),
            limit: query.limit.map(|l| l.to_string()).unwrap_or_default(),
            cursor: String::new(),
            refspec,
        })
        .await
        .locks;

    Ok(Json(LockResponse { locks }))
}


async fn create_lock(
    Path(path): Path<String>
) -> Result<StatusCode, ErrorResponse> {
    if !PathBuf::from(&path).exists() {
        return Err(ErrorResponse { error: format!("Path '{}' not found", path) });
    }

    let refspec = current_refspec()
        .ok_or_else(|| ErrorResponse { error: "Could not determine current ref".to_string() })?;

    let status = LFSClient::get()
        .await
        .lock(path.clone(), refspec)
        .await;

    match status {
        StatusCode::CREATED => Ok(StatusCode::CREATED),
        StatusCode::CONFLICT => Err(ErrorResponse { error: "Lock already exists".to_string() }),
        _ => Err(ErrorResponse { error: "Failed to create lock".to_string() }),
    }
}

#[derive(Debug, Deserialize)]
struct UnlockQuery {
    force: Option<bool>,
    id: Option<String>,
}

async fn remove_lock(
    Path(path): Path<String>,
    Query(query): Query<UnlockQuery>
) -> Result<StatusCode, ErrorResponse> {
    let refspec = current_refspec()
        .ok_or_else(|| ErrorResponse { error: "Could not determine current ref".to_string() })?;

    let id = match query.id {
        Some(id) => id,
        None => {
            let locks = LFSClient::get()
                .await
                .get_locks(LockListQuery {
                    refspec: refspec.clone(),
                    path: path.clone(),
                    id: String::new(),
                    cursor: String::new(),
                    limit: String::new(),
                })
                .await
                .locks;

            locks.first()
                .ok_or_else(|| ErrorResponse { error: "No lock found".to_string() })?
                .id.clone()
        }
    };

    let status = LFSClient::get()
        .await
        .unlock(id, refspec, query.force.unwrap_or(false))
        .await;

    match status {
        StatusCode::OK => Ok(StatusCode::NO_CONTENT),
        StatusCode::FORBIDDEN => Err(ErrorResponse { error: "Permission denied".to_string() }),
        _ => Err(ErrorResponse { error: "Failed to unlock".to_string() }),
    }
}




/// [different from `libra`].
/// Convert patterns to workdir.
fn convert_patterns_to_workdir(patterns: Vec<String>) -> Vec<String> {
    let mount_path = scorpio_config::workspace();
    let work_path = GPath::from(String::from(mount_path));
    patterns.into_iter().map(|p| {
        let mut w =work_path.clone();
        w.push(p);
        w.to_string()
    }).collect()
}

