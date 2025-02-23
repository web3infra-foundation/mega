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
            // 追踪LFS路径 (等价于track命令)
            .route("/attributes/track", post(track_lfs_path))
            // 取消追踪路径 (等价于untrack命令)
            .route("/attributes/untrack", post(untrack_lfs_path))
            // 列出当前分支锁定的文件 (等价于lfs locks)
            .route("/locks", get(list_locks))
            // 锁定文件 (等价于lfs lock) 
            .route("/locks/:path", post(create_lock))
            // 解锁文件 (等价于lfs unlock)
            .route("/locks/:path", delete(remove_lock))
            // 展示LFS文件信息 (等价于lfs ls-files)  
            .route("/objects/metadata", get(list_lfs_files))
        )
}

// 区域1: 属性管理端点 ===============================================

#[derive(Debug, Deserialize)]
struct TrackPathsRequest {
    patterns: Vec<String>,
}

async fn track_lfs_path(
    Json(payload): Json<TrackPathsRequest>
) -> Result<Json<HashMap<String, String>>, AppError> {
    // 业务逻辑：
    // 1. 更新.gitattributes文件
    // 2. 返回类似 {"status": "tracked", "added_paths": [...]}
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
    // 业务逻辑：从.gitattributes移除路径
    Ok(Json(HashMap::from([
        ("status".to_string(), "success".to_string()),
        ("removed_paths".to_string(), payload.paths.join(","))
    ])))
}

// 区域2: 文件锁管理端点 ============================================

#[derive(Debug, Deserialize)]
struct ListLocksQuery { // 对应CLI的三个选项参数
    id: Option<String>,
    path: Option<String>,
    limit: Option<u64>, 
}

#[derive(Debug, Serialize)]
struct LockInfo {
    id: String,
    path: String,
    owner: String,
    locked_at: i64, // 时间戳
}

async fn list_locks(
    Query(params): Query<ListLocksQuery>
) -> Result<Json<Vec<LockInfo>>, AppError> {
    // 业务逻辑：查询当前分支锁列表
    let mock_data = vec![LockInfo {
        id: "123".to_string(),
        path: params.path.unwrap_or_default(),
        owner: "user1".to_string(),
        locked_at: 1672531200
    }];
    Ok(Json(mock_data))
}

async fn create_lock(
    Path(path): Path<String> // 从URL路径获取文件路径
) -> Result<Json<LockInfo>, AppError> {
    // 业务逻辑：创建新锁
    Ok(Json(LockInfo {
        id: "456".to_string(),
        path,
        owner: "current_user".to_string(),
        locked_at: 1672531200
    }))
}

#[derive(Debug, Deserialize)]
struct UnlockParams { // CLI解锁参数
    force: bool,
    id: Option<String>,
}

async fn remove_lock(
    Path(path): Path<String>,
    Query(params): Query<UnlockParams>
) -> Result<Json<HashMap<String, String>>, AppError> {
    // 业务逻辑：强制或普通解锁
    Ok(Json(HashMap::from([
        ("status".to_string(), "unlocked".to_string()),
        ("path".to_string(), path),
        ("force_mode".to_string(), params.force.to_string())
    ])))
}

// 区域3: LFS文件信息查看端点 =======================================

#[derive(Debug, Deserialize)]
struct MetadataQueryParams { // 对应CLI选项参数
    long: Option<bool>,
    size: Option<bool>, 
    name_only: Option<bool>
}

#[derive(Debug, Serialize)]
struct LFSFileMeta {
    oid: String,
    symbolic_type: String, // "*"或"-"
    path: String,
    size_human: Option<String> // 可空字段
}

async fn list_lfs_files(
    Query(params): Query<MetadataQueryParams>
) -> Result<Json<Vec<LFSFileMeta>>, AppError> {
    // 业务逻辑：获取当前分支LFS文件列表
    let mock_file = LFSFileMeta {
        oid: "01ba4719...".to_string(),
        symbolic_type: "*".to_string(),
        path: "assets/image.png".to_string(),
        size_human: params.size.then(|| "15.2 MB".to_string())
    };
    Ok(Json(vec![mock_file]))
}

// 错误处理基础结构
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
