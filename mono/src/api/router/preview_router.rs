use std::{collections::HashMap, path::PathBuf};

use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};

use anyhow::Result;

use ceres::{
    api_service::ApiHandler,
    model::blame::{BlameQuery, BlameRequest, BlameResult},
    model::git::{
        BlobContentQuery, CodePreviewQuery, CreateEntryInfo, DiffPreviewPayload, EditFilePayload,
        EditFileResult, FileTreeItem, LatestCommitInfo, TreeCommitItem, TreeHashItem, TreeResponse,
    },
};
use common::model::CommonResult;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::{MonoApiServiceState, error::ApiError};
use crate::server::http_server::CODE_PREVIEW;
use ceres::users::get_org_member_by_username;
use tracing::{info, warn};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new()
        .routes(routes!(create_entry))
        .routes(routes!(get_latest_commit))
        .routes(routes!(get_tree_commit_info))
        .routes(routes!(get_file_blame))
        .routes(routes!(get_tree_content_hash))
        .routes(routes!(get_tree_dir_hash))
        .routes(routes!(path_can_be_cloned))
        .routes(routes!(get_tree_info))
        .routes(routes!(get_blob_string))
        .routes(routes!(preview_diff))
        .routes(routes!(save_edit))
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
    tag = CODE_PREVIEW
)]
async fn get_blob_string(
    Query(query): Query<BlobContentQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let data = state
        .api_handler(query.path.as_ref())
        .await?
        .get_blob_as_string(query.path.into(), Some(query.refs.as_str()))
        .await?;
    Ok(Json(CommonResult::success(data)))
}

/// Create file or folder in web UI
#[utoipa::path(
    post,
    path = "/create-entry",
    request_body = CreateEntryInfo,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = CODE_PREVIEW
)]
async fn create_entry(
    state: State<MonoApiServiceState>,
    headers: HeaderMap,
    Json(json): Json<CreateEntryInfo>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let handler = state.api_handler(json.path.as_ref()).await?;
    let commit_id = handler.create_monorepo_entry(json.clone()).await?;

    // If frontend provided author info, bind commit to that user (same as save_edit)
    // Guard for anonymous intent
    let username_trimmed_opt = json.author_username.as_deref().map(|s| s.trim());
    let is_anonymous = username_trimmed_opt
        .map(|u| u.is_empty() || u.eq_ignore_ascii_case("anonymous"))
        .unwrap_or(true);
    if let Some(email) = json.author_email.as_ref() {
        let stg = state.storage.commit_binding_storage();
        // Try resolve via organization member API when username and required headers are present
        let org_slug_opt = headers
            .get("X-Organization-Slug")
            .and_then(|v| v.to_str().ok());
        let cookie_header_opt = headers
            .get("X-Campsite-Session")
            .and_then(|v| v.to_str().ok())
            .map(|val| {
                let name = std::env::var("CAMPSITE_API_COOKIE_NAME")
                    .unwrap_or_else(|_| "_campsite_api_session".to_string());
                format!("{}={}", name, val)
            })
            .or_else(|| {
                headers
                    .get("X-Campsite-Cookie")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                headers
                    .get(axum::http::header::COOKIE)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            });
        let has_org = org_slug_opt.is_some();
        let has_cookie = cookie_header_opt.is_some();

        if let (Some(org_slug), Some(cookie_header)) = (org_slug_opt, cookie_header_opt) {
            if !is_anonymous {
                let username_trimmed = username_trimmed_opt.unwrap();
                match get_org_member_by_username(
                    state.storage.config(),
                    org_slug,
                    username_trimmed,
                    Some(cookie_header),
                )
                .await
                {
                    Ok(Some(member)) => {
                        info!(target:"commit_binding", commit_id=%commit_id, org_slug=org_slug, username=username_trimmed, "org-member verified (create-entry)");
                        let display = if member.display_name.is_empty() {
                            username_trimmed.to_string()
                        } else {
                            member.display_name
                        };
                        let avatar = if member.avatar_url.is_empty() {
                            None
                        } else {
                            Some(member.avatar_url)
                        };
                        stg.upsert_binding(
                            &commit_id,
                            email,
                            Some(username_trimmed.to_string()),
                            false,
                            Some(display),
                            avatar,
                        )
                        .await
                        .map_err(|e| {
                            ApiError::from(anyhow::anyhow!("Failed to save commit binding: {}", e))
                        })?;
                    }
                    _ => {
                        // Degrade to anonymous when verification fails
                        warn!(target:"commit_binding", commit_id=%commit_id, reason="verification_failed", "degrade to anonymous (create-entry)");
                        stg.upsert_binding(&commit_id, email, None, true, None, None)
                            .await
                            .map_err(|e| {
                                ApiError::from(anyhow::anyhow!(
                                    "Failed to save commit binding: {}",
                                    e
                                ))
                            })?;
                    }
                }
            } else {
                info!(target:"commit_binding", commit_id=%commit_id, reason="username_anonymous", "persist anonymous (create-entry)");
                stg.upsert_binding(&commit_id, email, None, true, None, None)
                    .await
                    .map_err(|e| {
                        ApiError::from(anyhow::anyhow!("Failed to save commit binding: {}", e))
                    })?;
            }
        } else {
            // Missing inputs, degrade to anonymous
            warn!(target:"commit_binding", commit_id=%commit_id, has_org_slug=has_org, has_cookie=has_cookie, reason="missing_headers_or_cookie", "degrade to anonymous (create-entry)");
            stg.upsert_binding(&commit_id, email, None, true, None, None)
                .await
                .map_err(|e| {
                    ApiError::from(anyhow::anyhow!("Failed to save commit binding: {}", e))
                })?;
        }
    }
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
    tag = CODE_PREVIEW
)]
async fn get_latest_commit(
    Query(query): Query<CodePreviewQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<LatestCommitInfo>, ApiError> {
    let query_path: std::path::PathBuf = query.path.into();
    let import_dir = state.storage.config().monorepo.import_dir.clone();
    if let Ok(rest) = query_path.strip_prefix(import_dir)
        && rest.components().count() == 1
    {
        let res = state
            .monorepo()
            .get_latest_commit(query_path.clone())
            .await?;
        return Ok(Json(res));
    }

    let res = state
        .api_handler(&query_path)
        .await?
        .get_latest_commit(query_path)
        .await?;
    Ok(Json(res))
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
    tag = CODE_PREVIEW
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
        let tree_items = handler
            .get_tree_info(path, Some(query.refs.as_str()))
            .await?;
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
        .get_tree_info(query.path.as_ref(), Some(query.refs.as_str()))
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
    tag = CODE_PREVIEW
)]
async fn get_tree_commit_info(
    Query(query): Query<CodePreviewQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<TreeCommitItem>>>, ApiError> {
    let data = state
        .api_handler(query.path.as_ref())
        .await?
        .get_tree_commit_info(query.path.into(), Some(query.refs.as_str()))
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
    tag = CODE_PREVIEW
)]
async fn get_tree_content_hash(
    Query(query): Query<CodePreviewQuery>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<TreeHashItem>>>, ApiError> {
    let data = state
        .api_handler(query.path.as_ref())
        .await?
        .get_tree_content_hash(query.path.into(), Some(query.refs.as_str()))
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
    tag = CODE_PREVIEW
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
        .get_tree_dir_hash(parent_path.into(), target_name, Some(query.refs.as_str()))
        .await?;

    Ok(Json(CommonResult::success(Some(data))))
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
    tag = CODE_PREVIEW
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

/// Get blame information for a file
#[utoipa::path(
    get,
    path = "/blame",
    params(
        BlameRequest
    ),
    responses(
        (status = 200, body = CommonResult<BlameResult>, content_type = "application/json")
    ),
    tag = CODE_PREVIEW
)]
async fn get_file_blame(
    Query(params): Query<BlameRequest>,
    State(state): State<MonoApiServiceState>,
) -> Result<Json<CommonResult<BlameResult>>, ApiError> {
    let ref_name = if params.refs.is_empty() {
        None
    } else {
        Some(params.refs.as_str())
    };

    let query = BlameQuery::from(&params);
    let result = state
        .api_handler(params.path.as_ref())
        .await?
        .get_file_blame(&params.path, ref_name, query)
        .await?;
    Ok(Json(CommonResult::success(Some(result))))
}

/// Preview unified diff for a single file before saving
#[utoipa::path(
    post,
    path = "/edit/diff-preview",
    request_body = DiffPreviewPayload,
    responses(
        (status = 200, body = CommonResult<neptune::model::diff_model::DiffItem>, content_type = "application/json")
    ),
    tag = CODE_PREVIEW
)]
async fn preview_diff(
    state: State<MonoApiServiceState>,
    Json(payload): Json<DiffPreviewPayload>,
) -> Result<Json<CommonResult<neptune::model::diff_model::DiffItem>>, ApiError> {
    let handler = state.api_handler(payload.path.as_ref()).await?;
    let item = handler.preview_file_diff(payload).await?;
    Ok(Json(CommonResult::success(item)))
}

/// Save edit and create a commit
#[utoipa::path(
    post,
    path = "/edit/save",
    request_body = EditFilePayload,
    responses(
        (status = 200, body = CommonResult<EditFileResult>, content_type = "application/json")
    ),
    tag = CODE_PREVIEW
)]
async fn save_edit(
    state: State<MonoApiServiceState>,
    headers: HeaderMap,
    Json(payload): Json<EditFilePayload>,
) -> Result<Json<CommonResult<EditFileResult>>, ApiError> {
    let handler = state.api_handler(payload.path.as_ref()).await?;
    let res = handler.save_file_edit(payload.clone()).await?;

    // If frontend provided author info, bind commit to that user
    if let Some(email) = payload.author_email.as_ref() {
        let stg = state.storage.commit_binding_storage();
        // Guard and normalize username
        let username_trimmed_opt = payload.author_username.as_deref().map(|s| s.trim());
        let is_anonymous = username_trimmed_opt
            .map(|u| u.is_empty() || u.eq_ignore_ascii_case("anonymous"))
            .unwrap_or(true);
        let org_slug_opt = headers
            .get("X-Organization-Slug")
            .and_then(|v| v.to_str().ok());
        let cookie_header_opt = headers
            .get("X-Campsite-Session")
            .and_then(|v| v.to_str().ok())
            .map(|val| {
                let name = std::env::var("CAMPSITE_API_COOKIE_NAME")
                    .unwrap_or_else(|_| "_campsite_api_session".to_string());
                format!("{}={}", name, val)
            })
            .or_else(|| {
                headers
                    .get("X-Campsite-Cookie")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                headers
                    .get(axum::http::header::COOKIE)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            });
        let has_org = org_slug_opt.is_some();
        let has_cookie = cookie_header_opt.is_some();

        if let (Some(org_slug), Some(cookie_header)) = (org_slug_opt, cookie_header_opt) {
            if !is_anonymous {
                let username_trimmed = username_trimmed_opt.unwrap();
                match get_org_member_by_username(
                    state.storage.config(),
                    org_slug,
                    username_trimmed,
                    Some(cookie_header),
                )
                .await
                {
                    Ok(Some(member)) => {
                        info!(target:"commit_binding", commit_id=%res.commit_id, org_slug=org_slug, username=username_trimmed, "org-member verified (save-edit)");
                        let display = if member.display_name.is_empty() {
                            username_trimmed.to_string()
                        } else {
                            member.display_name
                        };
                        let avatar = if member.avatar_url.is_empty() {
                            None
                        } else {
                            Some(member.avatar_url)
                        };
                        stg.upsert_binding(
                            &res.commit_id,
                            email,
                            Some(username_trimmed.to_string()),
                            false,
                            Some(display),
                            avatar,
                        )
                        .await
                        .map_err(|e| {
                            ApiError::from(anyhow::anyhow!("Failed to save commit binding: {}", e))
                        })?;
                    }
                    _ => {
                        warn!(target:"commit_binding", commit_id=%res.commit_id, reason="verification_failed", "degrade to anonymous (save-edit)");
                        stg.upsert_binding(&res.commit_id, email, None, true, None, None)
                            .await
                            .map_err(|e| {
                                ApiError::from(anyhow::anyhow!(
                                    "Failed to save commit binding: {}",
                                    e
                                ))
                            })?;
                    }
                }
            } else {
                info!(target:"commit_binding", commit_id=%res.commit_id, reason="username_anonymous", "persist anonymous (save-edit)");
                stg.upsert_binding(&res.commit_id, email, None, true, None, None)
                    .await
                    .map_err(|e| {
                        ApiError::from(anyhow::anyhow!("Failed to save commit binding: {}", e))
                    })?;
            }
        } else {
            warn!(target:"commit_binding", commit_id=%res.commit_id, has_org_slug=has_org, has_cookie=has_cookie, reason="missing_headers_or_cookie", "degrade to anonymous (save-edit)");
            stg.upsert_binding(&res.commit_id, email, None, true, None, None)
                .await
                .map_err(|e| {
                    ApiError::from(anyhow::anyhow!("Failed to save commit binding: {}", e))
                })?;
        }
    }

    Ok(Json(CommonResult::success(Some(res))))
}
