use axum::{
    routing::{get, post, delete},
    Router,
    extract::{Query, Path},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn create_app() -> Router {
    Router::new()
        .nest("/lfs", Router::new()
            // Track LFS paths (equivalent to the track command)
            .route("/attributes/track", post(track_lfs_path))
            // Untrack paths (equivalent to the untrack command)
            .route("/attributes/untrack", post(untrack_lfs_path))
            // List locked files in the current branch (equivalent to lfs locks)
            .route("/locks", get(list_locks))
            // Lock a file (equivalent to lfs lock) 
            .route("/locks/:path", post(create_lock))
            // Unlock a file (equivalent to lfs unlock)
            .route("/locks/:path", delete(remove_lock))
            // Display LFS file information (equivalent to lfs ls - files)  
            .route("/objects/metadata", get(list_lfs_files))
        )
}

// Region 1: Attribute management endpoints ===============================================

#[derive(Debug, Deserialize)]
struct TrackPathsRequest {
    patterns: Vec<String>,
}

async fn track_lfs_path(
    Json(payload): Json<TrackPathsRequest>
) -> Result<Json<HashMap<String, String>>, AppError> {
    // Business logic:
    // 1. Update the.gitattributes file
    // 2. Return something like {"status": "tracked", "added_paths": [...]}
    Ok(Json(HashMap::from([
        ("status".to_string(), "success".to_string()),
        ("added_paths".to_string(), payload.patterns.join(","))
    ])))    
}

#[derive(Debug, Deserialize)]
struct UntrackPathsRequest {
    paths: Vec<String>,
}

async fn untrack_lfs_path(
    Json(payload): Json<UntrackPathsRequest>
) -> Result<Json<HashMap<String, String>>, AppError> {
    // Business logic: Remove paths from.gitattributes
    Ok(Json(HashMap::from([
        ("status".to_string(), "success".to_string()),
        ("removed_paths".to_string(), payload.paths.join(","))
    ])))
}

// Region 2: File lock management endpoints ============================================

#[derive(Debug, Deserialize)]
struct ListLocksQuery { // Corresponds to the three option parameters of the CLI
    id: Option<String>,
    path: Option<String>,
    limit: Option<u64>, 
}

#[derive(Debug, Serialize)]
struct LockInfo {
    id: String,
    path: String,
    owner: String,
    locked_at: i64, // Timestamp
}

async fn list_locks(
    Query(params): Query<ListLocksQuery>
) -> Result<Json<Vec<LockInfo>>, AppError> {
    // Business logic: Query the list of locks in the current branch
    let mock_data = vec![LockInfo {
        id: "123".to_string(),
        path: params.path.unwrap_or_default(),
        owner: "user1".to_string(),
        locked_at: 1672531200
    }];
    Ok(Json(mock_data))
}

async fn create_lock(
    Path(path): Path<String> // Get the file path from the URL path
) -> Result<Json<LockInfo>, AppError> {
    // Business logic: Create a new lock
    Ok(Json(LockInfo {
        id: "456".to_string(),
        path,
        owner: "current_user".to_string(),
        locked_at: 1672531200
    }))
}

#[derive(Debug, Deserialize)]
struct UnlockParams { // CLI unlock parameters
    force: bool,
    id: Option<String>,
}

async fn remove_lock(
    Path(path): Path<String>,
    Query(params): Query<UnlockParams>
) -> Result<Json<HashMap<String, String>>, AppError> {
    // Business logic: Force or normal unlock
    Ok(Json(HashMap::from([
        ("status".to_string(), "unlocked".to_string()),
        ("path".to_string(), path),
        ("force_mode".to_string(), params.force.to_string())
    ])))
}

// Region 3: LFS file information viewing endpoints =======================================

#[derive(Debug, Deserialize)]
struct MetadataQueryParams { // Corresponds to the CLI option parameters
    long: Option<bool>,
    size: Option<bool>, 
    name_only: Option<bool>
}

#[derive(Debug, Serialize)]
struct LFSFileMeta {
    oid: String,
    symbolic_type: String, // "*" or "-"
    path: String,
    size_human: Option<String> // Nullable field
}

async fn list_lfs_files(
    Query(params): Query<MetadataQueryParams>
) -> Result<Json<Vec<LFSFileMeta>>, AppError> {
    // Business logic: Get the list of LFS files in the current branch
    let mock_file = LFSFileMeta {
        oid: "01ba4719...".to_string(),
        symbolic_type: "*".to_string(),
        path: "assets/image.png".to_string(),
        size_human: params.size.then(|| "15.2 MB".to_string())
    };
    Ok(Json(vec![mock_file]))
}

// Error handling infrastructure
#[derive(Debug)]
struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(HashMap::from([("error", self.0.to_string())]))
        ).into_response()
    }
}