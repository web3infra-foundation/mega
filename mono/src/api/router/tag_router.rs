use std::path::Path as StdPath;

use anyhow::anyhow;
use api_model::common::{CommonResult, PageParams};
use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::tag::{CreateTagRequest, DeleteTagResponse, TagListResponse, TagResponse};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{
        MonoApiServiceState,
        error::{ApiError, map_ceres_error},
    },
    server::http_server::TAG_MANAGE,
};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new()
        .routes(routes!(create_tag))
        .routes(routes!(list_tags))
        .routes(routes!(get_tag))
        .routes(routes!(delete_tag))
}

// Note: query-based path_context is intentionally removed for tag APIs; repo selection is
// resolved from router context (MonoApiServiceState) or request body for create if needed.

/// Resolve a target string (possibly "HEAD" or a commit hash) to an actual commit SHA.
/// If target_opt is Some and not "HEAD", return it directly. If it's None or "HEAD",
/// resolve to the repository's current HEAD/default branch commit.
async fn resolve_target_commit_id(
    state: &MonoApiServiceState,
    path_context: Option<&str>,
    target_opt: Option<&str>,
) -> Result<String, ApiError> {
    // if caller provided a specific non-"HEAD" target, use it directly
    if let Some(t) = target_opt
        && t != "HEAD"
        && !t.is_empty()
    {
        return Ok(t.to_string());
    }

    let import_dir = state.storage.config().monorepo.import_dir.clone();
    if let Some(path) = path_context {
        let std_path = StdPath::new(path);
        if std_path.starts_with(&import_dir) && std_path != StdPath::new(&import_dir) {
            // find repo model (longest-prefix match)
            if let Some(repo_model) = state
                .storage
                .git_db_storage()
                .find_git_repo_like_path(path)
                .await
                .map_err(|e| ApiError::from(anyhow!("Database error: {}", e)))?
            {
                let git = state.storage.git_db_storage();
                // try default branch ref
                if let Ok(Some(r)) = git.get_default_ref(repo_model.id).await {
                    return Ok(r.ref_git_id);
                }
                // fallback: any import ref for repo
                if let Ok(refs) = git.get_ref(repo_model.id).await
                    && let Some(r) = refs.into_iter().next()
                {
                    return Ok(r.ref_git_id);
                }
                return Ok("HEAD".to_string());
            }
            // If db lookup did not find a repo despite prefix, fall through to mono logic
        } else {
            // path is outside import_dir → mono
            let mono = state.storage.mono_storage();
            let resolved_path = path_context.unwrap_or("/");
            if let Ok(Some(r)) = mono.get_main_ref(resolved_path).await {
                return Ok(r.ref_commit_hash);
            }
            if let Ok(Some(root_ref)) = mono.get_main_ref("/").await {
                return Ok(root_ref.ref_commit_hash);
            }
            return Ok("HEAD".to_string());
        }
    }

    // Default fallback: try mono root ref
    let mono = state.storage.mono_storage();
    if let Ok(Some(root_ref)) = mono.get_main_ref("/").await {
        return Ok(root_ref.ref_commit_hash);
    }
    Ok("HEAD".to_string())
}

// Validate tag name against a conservative subset of Git ref rules.
fn validate_tag_name(name: &str) -> Result<(), ApiError> {
    // Basic checks that don't require iterating characters
    if name.is_empty() {
        return Err(ApiError::bad_request(anyhow!("Tag name must not be empty")));
    }

    if name.len() > 255 {
        return Err(ApiError::bad_request(anyhow!("Tag name is too long")));
    }

    if name.contains("..") || name.contains("@{") {
        return Err(ApiError::bad_request(anyhow!(
            "Tag name contains reserved sequence '..' or '@{{'"
        )));
    }

    if name.contains("//") {
        return Err(ApiError::bad_request(anyhow!(
            "Tag name must not contain '//'"
        )));
    }

    if name.ends_with(".lock") {
        return Err(ApiError::bad_request(anyhow!(
            "Tag name must not end with '.lock'"
        )));
    }

    // Single-pass character validation: forbidden chars, NUL, control chars
    let forbidden = [' ', '~', '^', ':', '?', '*', '[', '\\'];
    for c in name.chars() {
        if forbidden.contains(&c) {
            return Err(ApiError::bad_request(anyhow!(format!(
                "Tag name '{}' contains forbidden character '{}'",
                name, c
            ))));
        }
        if c == '\0' || c.is_control() {
            return Err(ApiError::bad_request(anyhow!(
                "Tag name contains invalid control characters"
            )));
        }
    }

    Ok(())
}

/// Create Tag
#[utoipa::path(
    post,
    path = "/tags",
    request_body(
        content = CreateTagRequest,
        content_type = "application/json"
    ),
    responses(
        (status = 201, body = CommonResult<TagResponse>, content_type = "application/json")
    ),
    tag = TAG_MANAGE
)]
async fn create_tag(
    State(state): State<MonoApiServiceState>,
    Json(req): Json<CreateTagRequest>,
) -> Result<Json<CommonResult<TagResponse>>, ApiError> {
    // We ignore query path_context for tag creation; use request target commit directly.
    validate_tag_name(&req.name)?;
    // Resolve target commit: if caller provided a target, use it; otherwise resolve using optional path_context.
    let resolved_target = if let Some(t) = req.target.as_deref() {
        if t != "HEAD" && !t.is_empty() {
            t.to_string()
        } else {
            // fallback: resolve using provided path_context if any
            resolve_target_commit_id(&state, req.path_context.as_deref(), None).await?
        }
    } else {
        resolve_target_commit_id(&state, req.path_context.as_deref(), None).await?
    };

    // dispatch to repo-specific handler via ApiHandler using path_context if provided
    let repo_path_ref = req.path_context.as_deref().unwrap_or("/");
    let api = state
        .api_handler(std::path::Path::new(repo_path_ref))
        .await
        .map_err(|e| map_ceres_error(e, "Failed to resolve api handler"))?;

    let tag_info = api
        .create_tag(
            Some(repo_path_ref.to_string()),
            req.name.clone(),
            Some(resolved_target),
            req.tagger_name.clone(),
            req.tagger_email.clone(),
            req.message.clone(),
        )
        .await
        .map_err(|e| map_ceres_error(e, "Failed to create tag"))?;

    let response = TagResponse {
        name: tag_info.name,
        tag_id: tag_info.tag_id,
        object_id: tag_info.object_id,
        object_type: tag_info.object_type,
        tagger: tag_info.tagger,
        message: tag_info.message,
        created_at: tag_info.created_at,
    };
    Ok(Json(CommonResult::success(Some(response))))
}

/// List all Tags
#[utoipa::path(
    post,
    path = "/tags/list",
    request_body = PageParams<String>,
    responses(
        (status = 200, body = CommonResult<TagListResponse>, content_type = "application/json")
    ),
    tag = TAG_MANAGE
)]

async fn list_tags(
    State(state): State<MonoApiServiceState>,
    Json(json): Json<PageParams<String>>,
) -> Result<Json<CommonResult<TagListResponse>>, ApiError> {
    let pagination = json.pagination;
    let repo_path_ref = if json.additional.trim().is_empty() {
        "/"
    } else {
        json.additional.as_str()
    };
    let api = state
        .api_handler(std::path::Path::new(repo_path_ref))
        .await
        .map_err(|e| map_ceres_error(e, "Failed to resolve api handler"))?;
    let (tags, total) = api
        .list_tags(Some(repo_path_ref.to_string()), pagination)
        .await
        .map_err(|e| map_ceres_error(e, "Failed to list tags"))?;
    let tag_responses: Vec<TagResponse> = tags
        .into_iter()
        .map(|t| TagResponse {
            name: t.name,
            tag_id: t.tag_id,
            object_id: t.object_id,
            object_type: t.object_type,
            tagger: t.tagger,
            message: t.message,
            created_at: t.created_at,
        })
        .collect();

    let response = TagListResponse {
        total,
        items: tag_responses,
    };
    Ok(Json(CommonResult::success(Some(response))))
}

/// Get Tag by name
#[utoipa::path(
    get,
    path = "/tags/{name}",
    responses(
        (status = 200, body = CommonResult<TagResponse>, content_type = "application/json"),
        (status = 404, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = TAG_MANAGE
)]
async fn get_tag(
    State(state): State<MonoApiServiceState>,
    Path(name): Path<String>,
) -> Result<Json<CommonResult<TagResponse>>, ApiError> {
    let repo_path = "/".to_string();
    let api = state
        .api_handler(std::path::Path::new(&repo_path))
        .await
        .map_err(|e| map_ceres_error(e, "Failed to resolve api handler"))?;

    match api
        .get_tag(Some(repo_path.clone()), name.clone())
        .await
        .map_err(|e| map_ceres_error(e, "Failed to get tag"))?
    {
        Some(t) => {
            let response = TagResponse {
                name: t.name,
                tag_id: t.tag_id,
                object_id: t.object_id,
                object_type: t.object_type,
                tagger: t.tagger,
                message: t.message,
                created_at: t.created_at,
            };
            Ok(Json(CommonResult::success(Some(response))))
        }
        None => Err(ApiError::not_found(anyhow!(format!(
            "Tag '{}' not found",
            name
        )))),
    }
}

/// Delete Tag
#[utoipa::path(
    delete,
    path = "/tags/{name}",
    responses(
        (status = 200, body = CommonResult<DeleteTagResponse>, content_type = "application/json"),
        (status = 404, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = TAG_MANAGE
)]
async fn delete_tag(
    State(state): State<MonoApiServiceState>,
    Path(name): Path<String>,
) -> Result<Json<CommonResult<DeleteTagResponse>>, ApiError> {
    let repo_path = "/".to_string(); // use root for delete operations by default
    let api = state
        .api_handler(std::path::Path::new(&repo_path))
        .await
        .map_err(|e| map_ceres_error(e, "Failed to resolve api handler"))?;
    api.delete_tag(Some(repo_path.clone()), name.clone())
        .await
        .map_err(|e| map_ceres_error(e, "Failed to delete tag"))?;

    let response = DeleteTagResponse {
        deleted_tag: name.clone(),
        message: format!("Tag '{}' successfully deleted", name),
    };
    Ok(Json(CommonResult::success(Some(response))))
}
