use axum::{
    extract::{Path, State},
    Json,
};
use common::model::CommonResult;
use mono::api::commit::model::{CommitBindingResponse, UserInfo, CommitBinding};
use mono::api::error::ApiError;
use utoipa_axum::{router::OpenApiRouter, routes};
use anyhow::anyhow;

use crate::api::MegaApiServiceState;

pub fn routers() -> OpenApiRouter<MegaApiServiceState> {
    OpenApiRouter::new().nest(
        "/commits",
        OpenApiRouter::new().routes(routes!(get_commit_binding)),
    )
}

/// Get commit binding information by commit SHA
#[utoipa::path(
    get,
    path = "/{sha}",
    params(
        ("sha" = String, Path, description = "Git commit SHA hash")
    ),
    responses(
        (status = 200, body = CommonResult<CommitBindingResponse>, content_type = "application/json"),
        (status = 404, description = "Commit binding not found")
    ),
    tag = "mega-commit"
)]
async fn get_commit_binding(
    State(state): State<MegaApiServiceState>,
    Path(sha): Path<String>,
) -> Result<Json<CommonResult<CommitBindingResponse>>, ApiError> {
    let commit_binding_storage = state.inner.storage.commit_binding_storage();
    let user_storage = state.inner.storage.user_storage();

    match commit_binding_storage.find_by_sha(&sha).await {
        Ok(Some(binding_model)) => {
            // Try to get user information if not anonymous
            let user_info = if !binding_model.is_anonymous && binding_model.matched_user_id.is_some() {
                let user_id_str = binding_model.matched_user_id.as_ref().unwrap();
                if let Ok(user_id) = user_id_str.parse::<i64>() {
                    if let Ok(Some(user)) = user_storage.find_user_by_id(user_id).await {
                        Some(UserInfo {
                            id: user.id.to_string(),
                            username: user.name.clone(),
                            display_name: Some(user.name.clone()),
                            avatar_url: Some(user.avatar_url.clone()),
                            email: user.email.clone(),
                        })
                    } else {
                        None
                    }
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
                matched_user_id: binding_model.matched_user_id,
                is_anonymous: binding_model.is_anonymous,
                matched_at: binding_model.matched_at.map(|dt| dt.and_utc().to_rfc3339()),
                created_at: binding_model.created_at.and_utc().to_rfc3339(),
                user: user_info.clone(),
            };

            // Prepare display information
            let (display_name, avatar_url, is_verified_user) = if binding_model.is_anonymous {
                ("匿名提交".to_string(), None, false)
            } else if let Some(ref user) = user_info {
                (
                    user.username.clone(), // Use username instead of display_name field
                    user.avatar_url.clone(),
                    true
                )
            } else {
                (binding_model.author_email.split('@').next().unwrap_or("未知用户").to_string(), None, false)
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
            display_name: "匿名提交".to_string(),
            avatar_url: None,
            is_verified_user: false,
        })))),
        Err(e) => {
            tracing::error!("Failed to query commit binding for {}: {}", sha, e);
            Err(ApiError::from(anyhow!(
                "Database query failed: {}",
                e
            )))
        }
    }
}
