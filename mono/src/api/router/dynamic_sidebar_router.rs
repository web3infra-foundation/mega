use axum::{
    Json,
    extract::{Path, State},
};
use ceres::{
    model::dynamic_sidebar::{
        CreateSidebarPayload, SidebarMenuListRes, SidebarRes, UpdateSidebarPayload,
    },
};
use common::model::CommonResult;
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

/// Delete sidebar menu
#[utoipa::path(
    delete,
    params(
        ("id", description = "Sidebar ID to delete"),
    ),                         
    path = "/{id}",
    responses(
        (status = 200, body = CommonResult<SidebarRes>, content_type = "application/json")
    ),
    tag = SIDEBAR_TAG                          
)]
async fn delete_sidebar_by_id(
    state: State<MonoApiServiceState>,
    Path(id): Path<i32>
) -> Result<Json<CommonResult<SidebarRes>>, ApiError>{
    let res = state.dynamic_sidebar_stg().delete_sidebar(id).await?;
    Ok(Json(CommonResult::success(Some(res.into()))))
}
