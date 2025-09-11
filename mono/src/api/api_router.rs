use std::{collections::HashMap, path::PathBuf};

use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    routing::get,
    Json,
};
use http::StatusCode;

use ceres::{
    api_service::ApiHandler,
    model::git::{
        BlobContentQuery, CodePreviewQuery, CreateFileInfo, FileTreeItem, LatestCommitInfo,
        TreeCommitItem, TreeHashItem, TreeQuery, TreeResponse, CommitBindingInfo,
    },
};
use common::model::CommonResult;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::{
    commit::commit_router, conversation::conv_router, error::ApiError, gpg::gpg_router, 
    issue::issue_router, label::label_router, mr::mr_router, notes::note_router, 
    user::user_router, MonoApiServiceState,
};
use crate::server::http_server::GIT_TAG;

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new()
        .routes(routes!(life_cycle_check))
        .routes(routes!(create_file))
        .routes(routes!(get_latest_commit))
        .routes(routes!(get_tree_commit_info))
        .routes(routes!(get_tree_content_hash))
        .routes(routes!(get_tree_dir_hash))
        .routes(routes!(path_can_be_cloned))
        .routes(routes!(get_tree_info))
        .routes(routes!(get_blob_string))
        .route("/file/blob/{object_id}", get(get_blob_file))
        .route("/file/tree", get(get_tree_file))
        .merge(mr_router::routers())
        .merge(gpg_router::routers())
        .merge(user_router::routers())
        .merge(issue_router::routers())
        .merge(label_router::routers())
        .merge(conv_router::routers())
        .merge(note_router::routers())
        .merge(commit_router::routers())
}

/// Get blob file as string
#[utoipa::path(
    get,
    params(
        BlobContentQuery
    ),
    path = "/blob",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = GIT_TAG
)]
async fn get_blob_string(
    Query(query): Query<BlobContentQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let data = state
        .api_handler(query.path.as_ref())
        .await?
        .get_blob_as_string(query.path.into())
        .await?;
    Ok(Json(CommonResult::success(data)))
}

/// Health Check
#[utoipa::path(
    get,
    path = "/status",
    responses(
        (status = 200, body = str, content_type = "text/plain")
    ),
    tag = GIT_TAG
)]
async fn life_cycle_check() -> Result<impl IntoResponse, ApiError> {
    Ok(Json("http ready"))
}

/// Create file in web UI
#[utoipa::path(
    post,
    path = "/create-file",
    request_body = CreateFileInfo,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = GIT_TAG
)]
async fn create_file(
    state: State<MonoApiServiceState>,
    Json(json): Json<CreateFileInfo>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .api_handler(json.path.as_ref())
        .await?
        .create_monorepo_file(json.clone())
        .await?;
    Ok(Json(CommonResult::success(None)))
}

/// Get latest commit by path
#[utoipa::path(
    get,
    path = "/latest-commit",
    params(
        CodePreviewQuery
    ),
    responses(
        (status = 200, body = LatestCommitInfo, content_type = "application/json")
    ),
    tag = GIT_TAG
)]
async fn get_latest_commit(
    Query(query): Query<CodePreviewQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<LatestCommitInfo>, ApiError> {
    let query_path: std::path::PathBuf = query.path.into();
    let import_dir = state.storage.config().monorepo.import_dir.clone(); 
    let mut commit_info = if let Ok(rest) = query_path.strip_prefix(import_dir) {
        if rest.components().count() == 1 {
            let res = state
                .monorepo()
                .get_latest_commit(query_path.clone())
                .await?;
            res
        } else {
            let res = state
                .api_handler(&query_path)
                .await?
                .get_latest_commit(query_path)
                .await?;
            res
        }
    } else {
        let res = state
            .api_handler(&query_path)
            .await?
            .get_latest_commit(query_path)
            .await?;
        res
    };

    // Query commit binding information
    let commit_binding_storage = state.storage.commit_binding_storage();
    let user_storage = state.storage.user_storage();
    
    if let Ok(Some(binding_model)) = commit_binding_storage.find_by_sha(&commit_info.oid).await {
        // Get user information if not anonymous
        let user_info = if !binding_model.is_anonymous && binding_model.matched_username.is_some() {
            let username = binding_model.matched_username.as_ref().unwrap();
            if let Ok(Some(user)) = user_storage.find_user_by_name(username).await {
                Some((user.name.clone(), user.avatar_url.clone()))
            } else {
                None
            }
        } else {
            None
        };

        let (display_name, avatar_url, is_verified_user) = if binding_model.is_anonymous {
            ("Anonymous".to_string(), None, false)
        } else if let Some((username, avatar)) = user_info {
            (username, Some(avatar), true)
        } else {
            (binding_model.author_email.split('@').next().unwrap_or(&binding_model.author_email).to_string(), None, false)
        };

        commit_info.binding_info = Some(CommitBindingInfo {
            matched_username: binding_model.matched_username,
            is_anonymous: binding_model.is_anonymous,
            is_verified_user,
            display_name,
            avatar_url,
            author_email: binding_model.author_email,
        });
    }

    Ok(Json(commit_info))
}

/// Get tree brief info
#[utoipa::path(
    get,
    path = "/tree",
    params(
        CodePreviewQuery
    ),
    responses(
        (status = 200, body = CommonResult<TreeResponse>, content_type = "application/json")
    ),
    tag = GIT_TAG
)]
async fn get_tree_info(
    Query(query): Query<CodePreviewQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<TreeResponse>>, ApiError> {
    let mut parts = Vec::new();

    let normalized_path = PathBuf::from(query.path.clone());
    let mut segments = normalized_path.components().peekable();
    let mut current = String::new();

    while let Some(segment) = segments.next() {
        let part = segment.as_os_str().to_string_lossy().to_string();
        if segments.peek().is_some() {
            if current != "/" && part != "/" {
                current.push('/');
            }
            current.push_str(&part);
            parts.push(current.clone());
        }
    }

    let mut file_tree = HashMap::new();

    for part in parts {
        let path = part.as_ref();
        let handler = state.api_handler(path).await?;
        let tree_items = handler.get_tree_info(path).await?;
        file_tree.insert(
            part,
            FileTreeItem {
                total_count: tree_items.len(),
                tree_items,
            },
        );
    }

    let tree_items = state
        .api_handler(query.path.as_ref())
        .await?
        .get_tree_info(query.path.as_ref())
        .await?;
    Ok(Json(CommonResult::success(Some(TreeResponse {
        file_tree,
        tree_items,
    }))))
}

/// List matching trees with commit msg by query
#[utoipa::path(
    get,
    path = "/tree/commit-info",
    params(
        CodePreviewQuery
    ),
    responses(
        (status = 200, body = CommonResult<Vec<TreeCommitItem>>, content_type = "application/json")
    ),
    tag = GIT_TAG
)]
async fn get_tree_commit_info(
    Query(query): Query<CodePreviewQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<TreeCommitItem>>>, ApiError> {
    let data = state
        .api_handler(query.path.as_ref())
        .await?
        .get_tree_commit_info(query.path.into())
        .await?;
    Ok(Json(CommonResult::success(Some(data))))
}

/// Get tree content hash,the dir's hash as same as old,file's hash is the content hash
#[utoipa::path(
    get,
    path = "/tree/content-hash",
    params(
        CodePreviewQuery
    ),
    responses(
        (status = 200, body = CommonResult<Vec<TreeHashItem>>, content_type = "application/json")
    ),
    tag = GIT_TAG
)]
async fn get_tree_content_hash(
    Query(query): Query<CodePreviewQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<TreeHashItem>>>, ApiError> {
    let data = state
        .api_handler(query.path.as_ref())
        .await?
        .get_tree_content_hash(query.path.into())
        .await?;
    Ok(Json(CommonResult::success(Some(data))))
}

/// return the dir's hash
#[utoipa::path(
    get,
    path = "/tree/dir-hash",
    params(
        CodePreviewQuery
    ),
    responses(
        (status = 200, body = CommonResult<Vec<TreeHashItem>>, content_type = "application/json")
    ),
    tag = GIT_TAG
)]
async fn get_tree_dir_hash(
    Query(query): Query<CodePreviewQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<TreeHashItem>>>, ApiError> {
    let path = std::path::Path::new(&query.path);
    let parent_path = path
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .to_string();
    let target_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    let data = state
        .api_handler(parent_path.as_ref())
        .await?
        .get_tree_dir_hash(parent_path.into(), target_name)
        .await?;

    Ok(Json(CommonResult::success(Some(data))))
}

pub async fn get_blob_file(
    state: State<MonoApiServiceState>,
    Path(oid): Path<String>,
) -> Result<Response, ApiError> {
    let api_handler = state.monorepo();

    let result = api_handler.get_raw_blob_by_hash(&oid).await.unwrap();
    let file_name = format!("inline; filename=\"{oid}\"");
    match result {
        Some(model) => Ok(Response::builder()
            .header("Content-Type", "application/octet-stream")
            .header("Content-Disposition", file_name)
            .body(Body::from(model.data.unwrap()))
            .unwrap()),
        None => Ok({
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()
        }),
    }
}

// Get tree as file
#[utoipa::path(
    get,
    path = "/file/tree",
    params(
        TreeQuery
    ),
    responses(
        (status = 200, body = CommonResult<Vec<TreeHashItem>>, content_type = "application/json")
    ),
    tag = GIT_TAG
)]
pub async fn get_tree_file(
    state: State<MonoApiServiceState>,
    Query(query): Query<TreeQuery>,
) -> Result<Response, ApiError> {
    let data = state
        .api_handler(query.path.as_ref())
        .await?
        .get_binary_tree_by_path(std::path::Path::new(&query.path), query.oid)
        .await?;

    let file_name = format!("inline; filename=\"{}\"", "");
    Ok(Response::builder()
        .header("Content-Type", "application/octet-stream")
        .header("Content-Disposition", file_name)
        .body(Body::from(data))
        .unwrap())
}

/// Check if a path can be cloned
#[utoipa::path(
    get,
    path = "/tree/path-can-clone",
    params(
        BlobContentQuery
    ),
    responses(
        (status = 200, body = CommonResult<bool>, content_type = "application/json")
    ),
    tag = GIT_TAG
)]
async fn path_can_be_cloned(
    Query(query): Query<BlobContentQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<bool>>, ApiError> {
    let path: PathBuf = query.path.clone().into();
    let import_dir = state.storage.config().monorepo.import_dir.clone();
    let res = if path.starts_with(&import_dir) {
        state
            .storage
            .git_db_storage()
            .find_git_repo_exact_match(path.to_str().unwrap())
            .await
            .unwrap()
            .is_some()
    } else {
        // any path under monorepo can be cloned
        true
    };
    Ok(Json(CommonResult::success(Some(res))))
}
