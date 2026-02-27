use api_model::common::CommonResult;
use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::group::UserEffectivePermissionResponse;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{
        MonoApiServiceState,
        api_common::group_permission::{
            build_user_effective_permission_response, parse_resource_context,
        },
        error::ApiError,
        oauth::model::LoginUser,
    },
    server::http_server::GROUP_PERMISSION_TAG,
};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/permissions",
        OpenApiRouter::new().routes(routes!(get_my_permission)),
    )
}

#[utoipa::path(
    get,
    path = "/me/{resource_type}/{resource_id}",
    params(
        ("resource_type" = String, Path, description = "Resource type, currently only `note`"),
        ("resource_id" = String, Path, description = "Resource ID")
    ),
    responses(
        (status = 200, body = CommonResult<UserEffectivePermissionResponse>),
        (status = 400, description = "Invalid resource_type or resource_id"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn get_my_permission(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path((resource_type, resource_id)): Path<(String, String)>,
) -> Result<Json<CommonResult<UserEffectivePermissionResponse>>, ApiError> {
    let actor = user.username;

    let (db_resource_type, resource_type_value, normalized_id) =
        parse_resource_context(&resource_type, &resource_id)?;

    let effective = state
        .monorepo()
        .get_user_effective_permission(&actor, db_resource_type, &normalized_id)
        .await?;

    let response = build_user_effective_permission_response(
        actor,
        resource_type_value,
        normalized_id,
        effective,
    );

    Ok(Json(CommonResult::success(Some(response))))
}
