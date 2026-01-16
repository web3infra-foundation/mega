use std::path::PathBuf;

use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::{
    change_list::MuiTreeNode,
    commit::{CommitBindingResponse, CommitFilesChangedPage, CommitHistoryParams, CommitSummary},
};
use common::model::{CommonPage, CommonResult, PageParams, Pagination};
use serde::{Deserialize, Serialize};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{MonoApiServiceState, error::ApiError},
    server::http_server::CODE_PREVIEW,
};

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateCommitBindingRequest {
    pub username: Option<String>,
    pub is_anonymous: bool,
}

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new()
        .routes(routes!(update_commit_binding))
        .routes(routes!(list_commit_history))
        .routes(routes!(commit_mui_tree))
        .routes(routes!(commit_files_changed))
}
/// Update commit binding information
#[utoipa::path(
    put,
    path = "/commits/{sha}/binding",
    params(
        ("sha" = String, Path, description = "Git commit SHA hash")
    ),
    request_body = UpdateCommitBindingRequest,
    responses(
        (status = 200, description = "Update commit binding information successfully",
         body = CommonResult<CommitBindingResponse>, content_type = "application/json"),
        (status = 404, description = "Commit not found"),
        (status = 400, description = "Invalid request")
    ),
    tag = CODE_PREVIEW
)]
#[axum::debug_handler]
async fn update_commit_binding(
    State(state): State<MonoApiServiceState>,
    Path(sha): Path<String>,
    Json(request): Json<UpdateCommitBindingRequest>,
) -> Result<Json<CommonResult<CommitBindingResponse>>, ApiError> {
    let commit_binding_storage = state.storage.commit_binding_storage();

    // Derive final username from request (ignore username when explicitly anonymous)
    let final_username = if request.is_anonymous {
        None
    } else {
        request.username.as_ref().and_then(|u| {
            let t = u.trim();
            if t.is_empty() || t.eq_ignore_ascii_case("anonymous") {
                None
            } else {
                Some(t.to_string())
            }
        })
    };

    // Update binding with simplified schema (no author_email)
    commit_binding_storage
        .upsert_binding(&sha, final_username.clone(), final_username.is_none())
        .await
        .map_err(|e| ApiError::from(anyhow::anyhow!("Failed to update binding: {}", e)))?;

    Ok(Json(CommonResult::success(Some(CommitBindingResponse {
        username: final_username,
    }))))
}

/// List commit history with optional refs, path filter, author filter, and pagination.
#[utoipa::path(
    post,
    path = "/commits/history",
    request_body = PageParams<CommitHistoryParams>,
    responses(
        (status = 200, description = "Commit history",
            body = CommonResult<CommonPage<CommitSummary>>, content_type = "application/json"),
    ),
    tag = CODE_PREVIEW
)]
#[axum::debug_handler]
async fn list_commit_history(
    State(state): State<MonoApiServiceState>,
    Json(req): Json<PageParams<CommitHistoryParams>>,
) -> Result<Json<CommonResult<CommonPage<CommitSummary>>>, ApiError> {
    // Build normalized absolute path from request input.
    let raw_path = if req.additional.path.is_empty() {
        PathBuf::from("/")
    } else {
        PathBuf::from(&req.additional.path)
    };
    let abs_path = if raw_path.has_root() {
        raw_path.clone()
    } else {
        PathBuf::from("/").join(raw_path)
    };

    // Determine repository selector separately from the filter path to avoid
    // treating a subdirectory as the repository root in import repos.
    // Try to resolve an import repo by looking up the repo model using the full path.
    let path_str = abs_path.to_str().ok_or_else(|| {
        ApiError::from(anyhow::anyhow!(
            "Path contains invalid UTF-8: {:?}",
            abs_path
        ))
    })?;

    let repo_selector = if let Ok(Some(model)) = state
        .storage
        .git_db_storage()
        .find_git_repo_like_path(path_str)
        .await
    {
        PathBuf::from(model.repo_path)
    } else {
        abs_path.clone()
    };

    // Create handler using the repository selector (repo root), not the subdirectory.
    let handler = state.api_handler(&repo_selector).await?;

    let refs_opt = if req.additional.refs.is_empty() {
        None
    } else {
        Some(req.additional.refs.as_str())
    };
    // Path filter: treat both empty string and "/" as root (None);
    // otherwise, use the absolute requested path as filter (Some(&abs_path)).
    let path_filter = if req.additional.path.is_empty() || req.additional.path == "/" {
        None
    } else {
        Some(&abs_path)
    };
    // Normalize author: treat empty/whitespace as None
    let author_opt = req
        .additional
        .author
        .as_deref()
        .map(|s| s.trim())
        .filter(|t| !t.is_empty());
    let (items, total) = handler
        .list_commit_history(refs_opt, path_filter, author_opt, req.pagination)
        .await?;
    Ok(Json(CommonResult::success(Some(CommonPage {
        items,
        total,
    }))))
}

/// Get commit changed files tree (MUI format)
#[utoipa::path(
    get,
    path = "/commits/{sha}/mui-tree",
    params(
        ("sha" = String, Path, description = "Commit SHA"),
        ("path" = String, Query, description = "Repository/Subrepo selector (required)")
    ),
    responses(
        (status = 200, description = "Commit changed files tree",
            body = CommonResult<Vec<MuiTreeNode>>, content_type = "application/json"),
        (status = 404, description = "Commit not found"),
    ),
    tag = CODE_PREVIEW
)]
#[axum::debug_handler]
async fn commit_mui_tree(
    State(state): State<MonoApiServiceState>,
    Path(sha): Path<String>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<CommonResult<Vec<MuiTreeNode>>>, ApiError> {
    let selector = resolve_selector_path(&q)?;
    let handler = state.api_handler(&selector).await?;
    let tree = handler.get_commit_mui_tree(&sha, &selector).await?;
    Ok(Json(CommonResult::success(Some(tree))))
}

/// Get paginated list of files changed in a commit, scoped by repository/subrepo selector
#[utoipa::path(
    post,
    path = "/commits/{sha}/files-changed",
    params(
        ("sha" = String, Path, description = "Commit SHA"),
        ("path" = String, Query, description = "Repository/Subrepo selector (required)")
    ),
    request_body = Pagination,
    responses(
        (status = 200, description = "Commit files changed page",
            body = CommonResult<CommitFilesChangedPage>, content_type = "application/json"),
        (status = 404, description = "Commit not found"),
    ),
    tag = CODE_PREVIEW
)]
#[axum::debug_handler]
async fn commit_files_changed(
    State(state): State<MonoApiServiceState>,
    Path(sha): Path<String>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
    Json(pagination): Json<Pagination>,
) -> Result<Json<CommonResult<CommitFilesChangedPage>>, ApiError> {
    let selector = resolve_selector_path(&q)?;
    let handler = state.api_handler(&selector).await?;
    let result = handler
        .get_commit_files_changed(&sha, &selector, pagination)
        .await?;
    Ok(Json(CommonResult::success(Some(result))))
}

fn resolve_selector_path(
    query: &std::collections::HashMap<String, String>,
) -> Result<PathBuf, ApiError> {
    let raw = query.get("path").cloned().ok_or_else(|| {
        ApiError::from(anyhow::anyhow!("Missing required query parameter 'path'"))
    })?;
    if raw.is_empty() {
        Ok(PathBuf::from("/"))
    } else {
        Ok(PathBuf::from(raw))
    }
}
