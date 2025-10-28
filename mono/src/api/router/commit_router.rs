use crate::api::{MonoApiServiceState, error::ApiError};
use crate::server::http_server::CODE_PREVIEW;
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use ceres::model::commit::CommitBindingResponse;
use ceres::users::get_org_member_by_username;
use common::model::CommonResult;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
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
    headers: HeaderMap,
    Json(request): Json<UpdateCommitBindingRequest>,
) -> Result<Json<CommonResult<CommitBindingResponse>>, ApiError> {
    let commit_binding_storage = state.storage.commit_binding_storage();

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

    // Resolve user via organization member API when not anonymous
    let mut resolved_username: Option<String> = None;
    let mut resolved_display_name: Option<String> = None;
    let mut resolved_avatar_url: Option<String> = None;
    let mut force_anonymous = false;

    if !request.is_anonymous {
        // Normalize and guard username; treat empty or "Anonymous" as anonymous intent
        let username_raw = request.username.clone().ok_or_else(|| {
            ApiError::from(anyhow::anyhow!("Username required when not anonymous"))
        })?;
        let username_trimmed = username_raw.trim();
        if username_trimmed.is_empty() || username_trimmed.eq_ignore_ascii_case("anonymous") {
            // Respect anonymous intent and skip remote lookup
            info!(
                target: "commit_binding",
                sha = %sha,
                reason = "username_anonymous",
                "skip org-member lookup and persist anonymous"
            );
            force_anonymous = true;
        } else {
            // Require organization slug header
            let org_slug = headers
                .get("X-Organization-Slug")
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| {
                    ApiError::from(anyhow::anyhow!("X-Organization-Slug header is required"))
                })?;

            // Build a Cookie header string for downstream request
            let cookie_header = headers
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
                })
                .ok_or_else(|| ApiError::from(anyhow::anyhow!("Session cookie not found")))?;

            match get_org_member_by_username(
                state.storage.config(),
                org_slug,
                username_trimmed,
                Some(cookie_header),
            )
            .await
            {
                Ok(Some(member)) => {
                    info!(
                        target: "commit_binding",
                        sha = %sha,
                        org_slug = org_slug,
                        username = username_trimmed,
                        "org-member verified"
                    );
                    resolved_username = Some(username_trimmed.to_string());
                    resolved_display_name = Some(if member.display_name.is_empty() {
                        username_trimmed.to_string()
                    } else {
                        member.display_name
                    });
                    resolved_avatar_url = if member.avatar_url.is_empty() {
                        None
                    } else {
                        Some(member.avatar_url)
                    };
                }
                Ok(None) => {
                    warn!(
                        target: "commit_binding",
                        sha = %sha,
                        org_slug = org_slug,
                        username = username_trimmed,
                        "org-member not found"
                    );
                    return Err(ApiError::from(anyhow::anyhow!(
                        "User not found in organization"
                    )));
                }
                Err(e) => {
                    error!(
                        target: "commit_binding",
                        sha = %sha,
                        org_slug = org_slug,
                        username = username_trimmed,
                        error = %e,
                        "org-member lookup failed"
                    );
                    return Err(ApiError::from(anyhow::anyhow!(
                        "get_org_member_by_username failed: {}",
                        e
                    )));
                }
            }
        }
    }

    // Update the binding (persist presentation fields when available)
    commit_binding_storage
        .upsert_binding(
            &sha,
            &author_email,
            if request.is_anonymous || force_anonymous {
                None
            } else {
                resolved_username.clone()
            },
            request.is_anonymous || force_anonymous || resolved_username.is_none(),
            resolved_display_name.clone(),
            resolved_avatar_url.clone(),
        )
        .await
        .map_err(|e| ApiError::from(anyhow::anyhow!("Failed to update binding: {}", e)))?;

    // Prepare response with updated information from resolved data only
    let effective_anonymous =
        request.is_anonymous || force_anonymous || resolved_username.is_none();
    let (display_name, avatar_url, is_verified_user) = if effective_anonymous {
        ("Anonymous".to_string(), None, false)
    } else {
        (
            resolved_display_name.unwrap_or_default(),
            resolved_avatar_url,
            true,
        )
    };

    // Return success response with complete information
    Ok(Json(CommonResult::success(Some(CommitBindingResponse {
        display_name,
        avatar_url,
        is_verified_user,
    }))))
}
