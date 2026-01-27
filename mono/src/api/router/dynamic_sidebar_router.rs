use api_model::common::CommonResult;
use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::dynamic_sidebar::{
    CreateSidebarPayload, SidebarMenuListRes, SidebarRes, SidebarSyncPayload, UpdateSidebarPayload,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{MonoApiServiceState, error::ApiError},
    server::http_server::SIDEBAR_TAG,
};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/sidebar",
        OpenApiRouter::new()
            .routes(routes!(sidebar_menu_list))
            .routes(routes!(new_sidebar))
            .routes(routes!(update_sidebar_by_id))
            .routes(routes!(delete_sidebar_by_id))
            .routes(routes!(sync_sidebar)),
    )
}

/// Get all sidebar menu
#[utoipa::path(
    get,
    path = "/list",
    responses(
        (status = 200, body = CommonResult<SidebarMenuListRes>, content_type = "application/json")
    ),
    tag = SIDEBAR_TAG
)]
async fn sidebar_menu_list(
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<SidebarMenuListRes>>, ApiError> {
    let items: SidebarMenuListRes = state
        .dynamic_sidebar_stg()
        .get_sidebars()
        .await?
        .into_iter()
        .map(|m| m.into())
        .collect();
    Ok(Json(CommonResult::success(Some(items))))
}

/// New sidebar menu
#[utoipa::path(
    post,
    path = "/new",
    request_body = CreateSidebarPayload,
    responses(
        (status = 200, body = CommonResult<SidebarRes>, content_type = "application/json")
    ),
    tag = SIDEBAR_TAG
)]
async fn new_sidebar(
    state: State<MonoApiServiceState>,
    Json(json): Json<CreateSidebarPayload>,
) -> Result<Json<CommonResult<SidebarRes>>, ApiError> {
    let res = state
        .dynamic_sidebar_stg()
        .new_sidebar(
            json.public_id,
            json.label,
            json.href,
            json.visible,
            json.order_index,
        )
        .await?;
    Ok(Json(CommonResult::success(Some(res.into()))))
}

/// Update sidebar menu
#[utoipa::path(
    post,
    params(
        ("id", description = "Sidebar ID to update"),
    ),
    path = "/update/{id}",
    request_body = UpdateSidebarPayload,
    responses(
        (status = 200, body = CommonResult<SidebarRes>, content_type = "application/json")
    ),
    tag = SIDEBAR_TAG
)]
async fn update_sidebar_by_id(
    state: State<MonoApiServiceState>,
    Path(id): Path<i32>,
    Json(json): Json<UpdateSidebarPayload>,
) -> Result<Json<CommonResult<SidebarRes>>, ApiError> {
    let res = state
        .dynamic_sidebar_stg()
        .update_sidebar(
            id,
            json.public_id,
            json.label,
            json.href,
            json.visible,
            json.order_index,
        )
        .await?;

    Ok(Json(CommonResult::success(Some(res.into()))))
}

/// Sync sidebar menus
#[utoipa::path(
    post,
    path = "/sync",
    request_body = Vec<SidebarSyncPayload>,
    responses(
        (status = 200, body = CommonResult<Vec<SidebarRes>>, content_type = "application/json")
    ),
    tag = SIDEBAR_TAG,
    description = "Sync sidebar menus. \
Each `public_id` and `order_index` must be unique across all sidebar items. \
The operation will fail if: \
- A new item has a `public_id` that already exists \
- An update tries to set a `public_id` to one that's already in use by another item \
- Multiple items in the payload have the same `order_index` \
- An update tries to set an `order_index` that's already in use \
The transaction will be rolled back if any of these constraints are violated."
)]
async fn sync_sidebar(
    state: State<MonoApiServiceState>,
    Json(payloads): Json<Vec<SidebarSyncPayload>>,
) -> Result<Json<CommonResult<Vec<SidebarRes>>>, ApiError> {
    let res = state
        .dynamic_sidebar_stg()
        .sync_sidebar(payloads.into_iter().map(|item| item.into()).collect())
        .await?;

    Ok(Json(CommonResult::success(Some(
        res.into_iter().map(|item| item.into()).collect(),
    ))))
}

/// Delete sidebar menu
#[utoipa::path(
    delete,
    params(
        ("id", description = "Sidebar ID to delete")
    ),
    path = "/{id}",
    responses(
        (status = 200, body = CommonResult<SidebarRes>, content_type = "application/json")
    ),
    tag = SIDEBAR_TAG
)]
async fn delete_sidebar_by_id(
    state: State<MonoApiServiceState>,
    Path(id): Path<i32>,
) -> Result<Json<CommonResult<SidebarRes>>, ApiError> {
    let res = state.dynamic_sidebar_stg().delete_sidebar(id).await?;
    Ok(Json(CommonResult::success(Some(res.into()))))
}
