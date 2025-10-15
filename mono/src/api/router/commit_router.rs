use crate::api::{MonoApiServiceState, error::ApiError};
use crate::server::http_server::CODE_PREVIEW;
use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::commit::CommitBindingResponse;
use common::model::CommonResult;
use serde::{Deserialize, Serialize};
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateCommitBindingRequest {
    pub username: Option<String>,
    pub is_anonymous: bool,
}

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().routes(routes!(update_commit_binding))
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
    let user_storage = state.storage.user_storage();

    // First check if commit binding exists
    let existing_binding = commit_binding_storage
        .find_by_sha(&sha)
        .await
        .map_err(|e| ApiError::from(anyhow::anyhow!("Database query failed: {}", e)))?;

    let author_email = if let Some(ref binding) = existing_binding {
        binding.author_email.clone()
    } else {
        // If no binding exists, we need the author email - this could be passed in request or derived from git
        return Err(ApiError::from(anyhow::anyhow!(
            "No existing binding found for commit {}",
            sha
        )));
    };

    // Validate user if not anonymous
    if !request.is_anonymous {
        if let Some(ref username) = request.username {
            let user_exists = user_storage
                .find_user_by_name(username)
                .await
                .map_err(|e| ApiError::from(anyhow::anyhow!("User validation failed: {}", e)))?;

            if user_exists.is_none() {
                return Err(ApiError::from(anyhow::anyhow!(
                    "User not found: {}",
                    username
                )));
            }
        } else {
            return Err(ApiError::from(anyhow::anyhow!(
                "Username required when not anonymous"
            )));
        }
    }

    // Update the binding
    commit_binding_storage
        .upsert_binding(
            &sha,
            &author_email,
            request.username.clone(),
            request.is_anonymous,
        )
        .await
        .map_err(|e| ApiError::from(anyhow::anyhow!("Failed to update binding: {}", e)))?;

    // Prepare response with updated information
    let (display_name, avatar_url, is_verified_user) = if request.is_anonymous {
        ("Anonymous".to_string(), None, false)
    } else if let Some(ref username) = request.username {
        // Get user info for verified response
        match user_storage.find_user_by_name(username).await {
            Ok(Some(user)) => (user.name.clone(), Some(user.avatar_url.clone()), true),
            _ => (username.clone(), None, true),
        }
    } else {
        ("Anonymous".to_string(), None, false)
    };

    // Return success response with complete information
    Ok(Json(CommonResult::success(Some(CommitBindingResponse {
        display_name,
        avatar_url,
        is_verified_user,
    }))))
}
