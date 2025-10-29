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
