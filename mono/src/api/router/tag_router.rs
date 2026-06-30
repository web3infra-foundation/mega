use api_model::common::{CommonResult, PageParams};
use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::tag::{CreateTagRequest, DeleteTagResponse, TagListResponse, TagResponse};
use common::errors::MegaError;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::{MonoApiServiceState, api_doc::TAG_MANAGE, error::ApiError};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new()
        .routes(routes!(create_tag))
        .routes(routes!(list_tags))
        .routes(routes!(get_tag))
        .routes(routes!(delete_tag))
}

/// Resolve a target string (possibly "HEAD" or a commit hash) to an actual commit SHA.
async fn resolve_target_commit_id(
    state: &MonoApiServiceState,
    path_context: Option<&str>,
    target_opt: Option<&str>,
) -> Result<String, ApiError> {
    Ok(state
        .monorepo()
        .resolve_target_commit_id(path_context, target_opt)
        .await?)
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
        (status = 201, body = CommonResult<TagResponse>, content_type = "application/json"),
        (status = 400, description = "Invalid tag name or request", content_type = "application/json"),
        (status = 404, description = "Target commit not found", content_type = "application/json"),
    ),
    tag = TAG_MANAGE
)]
async fn create_tag(
    State(state): State<MonoApiServiceState>,
    Json(req): Json<CreateTagRequest>,
) -> Result<Json<CommonResult<TagResponse>>, ApiError> {
    let resolved_target = if let Some(t) = req.target.as_deref() {
        if t != "HEAD" && !t.is_empty() {
            t.to_string()
        } else {
            resolve_target_commit_id(&state, req.path_context.as_deref(), None).await?
        }
    } else {
        resolve_target_commit_id(&state, req.path_context.as_deref(), None).await?
    };

    let repo_path_ref = req.path_context.as_deref().unwrap_or("/");
    let api = state
        .api_handler(std::path::Path::new(repo_path_ref))
        .await?;

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
        .map_err(MegaError::from)?;

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
        .await?;
    let (tags, total) = api
        .list_tags(Some(repo_path_ref.to_string()), pagination)
        .await
        .map_err(MegaError::from)?;
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
    let api = state.api_handler(std::path::Path::new(&repo_path)).await?;

    match api
        .get_tag(Some(repo_path.clone()), name.clone())
        .await
        .map_err(MegaError::from)?
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
        None => Err(MegaError::NotFound(format!("Tag '{name}' not found")).into()),
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
    let repo_path = "/".to_string();
    let api = state.api_handler(std::path::Path::new(&repo_path)).await?;
    api.delete_tag(Some(repo_path.clone()), name.clone())
        .await
        .map_err(MegaError::from)?;

    let response = DeleteTagResponse {
        deleted_tag: name.clone(),
        message: format!("Tag '{}' successfully deleted", name),
    };
    Ok(Json(CommonResult::success(Some(response))))
}
