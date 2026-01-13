//! Admin-related API endpoints.
//!
//! Provides endpoints for admin permission checks with path context support:
//! - `GET /api/v1/admin/me` - Check if current user is admin (optional `path` query param)
//! - `GET /api/v1/admin/list` - List all admins (admin-only, optional `path` query param)
//!
//! # Path Context
//! The `path` query parameter determines which root directory's admin list to check.
//! If not provided, defaults to `/project`.
//!
//! Examples:
//! - `GET /api/v1/admin/me?path=/doc` - Check if user is admin for `/doc`
//! - `GET /api/v1/admin/list?path=/release` - List admins for `/release`
//!
//! # Auth Behavior
//! - 401 Unauthorized: No valid session (handled by `LoginUser` extractor)
//! - 403 Forbidden: Logged in but not admin (for `/list` endpoint)

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::MonoApiServiceState;
use crate::api::error::ApiError;
use crate::api::oauth::model::LoginUser;
use crate::server::http_server::USER_TAG;
use ceres::api_service::admin_ops;
use common::model::CommonResult;

/// Default path when not specified in query params.
const DEFAULT_PATH: &str = "/project";

/// Query parameters for admin endpoints.
#[derive(Debug, Deserialize, IntoParams)]
pub struct AdminQueryParams {
    /// Path context to determine which root directory's admin list to check.
    /// Defaults to `/project` if not provided.
    #[param(example = "/project")]
    pub path: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct IsAdminResponse {
    pub is_admin: bool,
    pub root_dir: String,
}

#[derive(Serialize, ToSchema)]
pub struct AdminListResponse {
    pub admins: Vec<String>,
    pub root_dir: String,
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
/// Returns whether the current user is an admin for the specified path context.
/// If no path is provided, defaults to `/project`.
#[utoipa::path(
    get,
    path = "/me",
    params(AdminQueryParams),
    responses(
        (status = 200, body = CommonResult<IsAdminResponse>),
        (status = 401, description = "Unauthorized"),
    ),
    tag = USER_TAG
)]
async fn is_admin_me(
    user: LoginUser,
    Query(params): Query<AdminQueryParams>,
    State(state): State<MonoApiServiceState>,
) -> Result<Json<CommonResult<IsAdminResponse>>, ApiError> {
    let path = params.path.as_deref().unwrap_or(DEFAULT_PATH);
    let root_dir = admin_ops::extract_root_dir(path);

    let is_admin = state
        .monorepo()
        .check_is_admin(&user.username, path)
        .await?;

    Ok(Json(CommonResult::success(Some(IsAdminResponse {
        is_admin,
        root_dir,
    }))))
}

/// GET /api/v1/admin/list
///
/// Returns a list of all admin usernames for the specified path context.
/// Only admins for that path can access this endpoint.
#[utoipa::path(
    get,
    path = "/list",
    params(AdminQueryParams),
    responses(
        (status = 200, body = CommonResult<AdminListResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not admin for this path"),
    ),
    tag = USER_TAG
)]
async fn admin_list(
    user: LoginUser,
    Query(params): Query<AdminQueryParams>,
    State(state): State<MonoApiServiceState>,
) -> Result<Json<CommonResult<AdminListResponse>>, ApiError> {
    let path = params.path.as_deref().unwrap_or(DEFAULT_PATH);
    let root_dir = admin_ops::extract_root_dir(path);

    // User must be admin for this path to view the admin list
    if !state
        .monorepo()
        .check_is_admin(&user.username, path)
        .await?
    {
        return Err(ApiError::with_status(
            http::StatusCode::FORBIDDEN,
            anyhow::anyhow!("Admin access required for path: {}", path),
        ));
    }

    let admins = state.monorepo().get_all_admins(path).await?;

    Ok(Json(CommonResult::success(Some(AdminListResponse {
        admins,
        root_dir,
    }))))
}
