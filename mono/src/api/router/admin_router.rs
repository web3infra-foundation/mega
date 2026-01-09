//! Admin-related API endpoints.
//!
//! Provides endpoints for admin permission checks:
//! - `GET /api/v1/admin/me` - Check if current user is admin
//! - `GET /api/v1/admin/list` - List all admins (admin-only)
//!
//! # Auth Behavior
//! - 401 Unauthorized: No valid session (handled by `LoginUser` extractor)
//! - 403 Forbidden: Logged in but not admin (for `/list` endpoint)

use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::MonoApiServiceState;
use crate::api::error::ApiError;
use crate::api::oauth::model::LoginUser;
use crate::server::http_server::USER_TAG;
use common::model::CommonResult;

/// Response for `/api/v1/admin/me`.
#[derive(Serialize, ToSchema)]
pub struct IsAdminResponse {
    pub is_admin: bool,
}

/// Response for `/api/v1/admin/list`.
#[derive(Serialize, ToSchema)]
pub struct AdminListResponse {
    pub admins: Vec<String>,
}

/// Build the admin router.
pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/admin",
        OpenApiRouter::new()
            .routes(routes!(is_admin_me))
            .routes(routes!(admin_list)),
    )
}

/// GET /api/v1/admin/me
///
/// Returns whether the current user is an admin based on Cedar entity data.
#[utoipa::path(
    get,
    path = "/me",
    responses(
        (status = 200, body = CommonResult<IsAdminResponse>),
        (status = 401, description = "Unauthorized"),
    ),
    tag = USER_TAG
)]
async fn is_admin_me(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
) -> Result<Json<CommonResult<IsAdminResponse>>, ApiError> {
    let is_admin = state.monorepo().check_is_admin(&user.username).await?;
    Ok(Json(CommonResult::success(Some(IsAdminResponse {
        is_admin,
    }))))
}

/// GET /api/v1/admin/list
///
/// Returns a list of all admin usernames. Only admins can access this endpoint.
#[utoipa::path(
    get,
    path = "/list",
    responses(
        (status = 200, body = CommonResult<AdminListResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not admin"),
    ),
    tag = USER_TAG
)]
async fn admin_list(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
) -> Result<Json<CommonResult<AdminListResponse>>, ApiError> {
    if !state.monorepo().check_is_admin(&user.username).await? {
        return Err(ApiError::with_status(
            http::StatusCode::FORBIDDEN,
            anyhow::anyhow!("Admin access required"),
        ));
    }
    let admins = state.monorepo().get_all_admins().await?;
    Ok(Json(CommonResult::success(Some(AdminListResponse {
        admins,
    }))))
}
