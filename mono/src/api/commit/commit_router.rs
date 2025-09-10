use super::model::{CommitBinding, CommitBindingResponse, UserInfo};
use crate::api::{error::ApiError, MonoApiServiceState};
use crate::server::http_server::GIT_TAG;
use axum::{
    extract::{Path, State},
    Json,
};
use common::model::CommonResult;
use utoipa_axum::{router::OpenApiRouter, routes};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateCommitBindingRequest {
    pub username: Option<String>,
    pub is_anonymous: bool,
}

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new()
        .routes(routes!(get_commit_binding))
        .routes(routes!(update_commit_binding))
}

/// Get commit binding information by commit SHA
#[utoipa::path(
    get,
    path = "/commits/{sha}/binding",
    params(
        ("sha" = String, Path, description = "Git commit SHA hash")
    ),
    responses(
        (status = 200, description = "Get commit binding information successfully", 
         body = CommonResult<CommitBindingResponse>, content_type = "application/json"),
        (status = 404, description = "Commit binding not found")
    ),
    tag = GIT_TAG
)]
#[axum::debug_handler]
async fn get_commit_binding(
    State(state): State<MonoApiServiceState>,
    Path(sha): Path<String>,
) -> Result<Json<CommonResult<CommitBindingResponse>>, ApiError> {
    let commit_binding_storage = state.storage.commit_binding_storage();
    let user_storage = state.storage.user_storage();

    match commit_binding_storage.find_by_sha(&sha).await {
        Ok(Some(binding_model)) => {
            // Try to get user information if not anonymous
            let user_info = if !binding_model.is_anonymous && binding_model.matched_username.is_some() {
                let username = binding_model.matched_username.as_ref().unwrap();
                if let Ok(Some(user)) = user_storage.find_user_by_name(username).await {
                    Some(UserInfo {
                        id: user.id.to_string(),
                        username: user.name.clone(),
                        display_name: Some(user.name.clone()), // Use name as display_name since display_name field doesn't exist
                        avatar_url: Some(user.avatar_url.clone()),
                        email: user.email.clone(),
                    })
                } else {
                    None
                }
            } else {
                None
            };

            let binding = CommitBinding {
                id: binding_model.id,
                commit_sha: binding_model.commit_sha,
                author_email: binding_model.author_email.clone(),
                matched_username: binding_model.matched_username,
                is_anonymous: binding_model.is_anonymous,
                matched_at: binding_model.matched_at.map(|dt| dt.and_utc().to_rfc3339()),
                created_at: binding_model.created_at.and_utc().to_rfc3339(),
                user: user_info.clone(),
            };

            // Prepare display information
            let (display_name, avatar_url, is_verified_user) = if binding_model.is_anonymous {
                ("Anonymous".to_string(), None, false)
            } else if let Some(ref user) = user_info {
                (
                    user.display_name.clone().unwrap_or(user.username.clone()),
                    user.avatar_url.clone(),
                    true
                )
            } else {
                // Fallback for cases where user is matched but user info is not available
                (binding_model.author_email.split('@').next().unwrap_or(&binding_model.author_email).to_string(), None, false)
            };

            Ok(Json(CommonResult::success(Some(CommitBindingResponse {
                binding: Some(binding),
                display_name,
                avatar_url,
                is_verified_user,
            }))))
        }
        Ok(None) => Ok(Json(CommonResult::success(Some(CommitBindingResponse {
            binding: None,
            display_name: "Anonymous".to_string(),
            avatar_url: None,
            is_verified_user: false,
        })))),
        Err(e) => {
            tracing::error!("Failed to query commit binding for {}: {}", sha, e);
            Err(ApiError::from(anyhow::anyhow!(
                "Database query failed: {}",
                e
            )))
        }
    }
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
    tag = GIT_TAG
)]
#[axum::debug_handler]
async fn update_commit_binding(
    State(state): State<MonoApiServiceState>,
    Path(sha): Path<String>,
    Json(request): Json<UpdateCommitBindingRequest>,
) -> Result<Json<CommonResult<CommitBindingResponse>>, ApiError> {
    let commit_binding_storage = state.storage.commit_binding_storage();
    let user_storage = state.storage.user_storage();

    // First check if commit binding exists
    let existing_binding = commit_binding_storage.find_by_sha(&sha).await
        .map_err(|e| ApiError::from(anyhow::anyhow!("Database query failed: {}", e)))?;

    let author_email = if let Some(ref binding) = existing_binding {
        binding.author_email.clone()
    } else {
        // If no binding exists, we need the author email - this could be passed in request or derived from git
        return Err(ApiError::from(anyhow::anyhow!("No existing binding found for commit {}", sha)));
    };

    // Validate user if not anonymous
    if !request.is_anonymous {
        if let Some(ref username) = request.username {
            let user_exists = user_storage.find_user_by_name(username).await
                .map_err(|e| ApiError::from(anyhow::anyhow!("User validation failed: {}", e)))?;
            
            if user_exists.is_none() {
                return Err(ApiError::from(anyhow::anyhow!("User not found: {}", username)));
            }
        } else {
            return Err(ApiError::from(anyhow::anyhow!("Username required when not anonymous")));
        }
    }

    // Update the binding
    commit_binding_storage.upsert_binding(
        &sha,
        &author_email,
        request.username.clone(),
        request.is_anonymous,
    ).await
    .map_err(|e| ApiError::from(anyhow::anyhow!("Failed to update binding: {}", e)))?;

    // Return updated binding information
    get_commit_binding(State(state), Path(sha)).await
}
