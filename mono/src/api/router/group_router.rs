use anyhow::anyhow;
use api_model::common::{CommonPage, CommonResult, PageParams, Pagination};
use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::group::{
    AddMembersRequest, CreateGroupRequest, DeleteGroupResponse, DeletePermissionsResponse,
    EmptyListAdditional, GroupMemberResponse, GroupResponse, RemoveMemberResponse,
    ResourcePermissionResponse, SetPermissionsRequest, UserEffectivePermissionResponse,
    UserGroupsResponse,
};
use jupiter::model::group_dto::{CreateGroupPayload, ResourcePermissionBinding};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{
        MonoApiServiceState,
        api_common::group_permission::{
            build_user_effective_permission_response, ensure_admin, parse_resource_context,
        },
        error::ApiError,
        oauth::model::LoginUser,
    },
    server::http_server::GROUP_PERMISSION_TAG,
};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/admin",
        OpenApiRouter::new()
            .routes(routes!(create_group))
            .routes(routes!(list_groups))
            .routes(routes!(get_group))
            .routes(routes!(delete_group))
            .routes(routes!(add_group_members))
            .routes(routes!(remove_group_member))
            .routes(routes!(list_group_members))
            .routes(routes!(set_resource_permissions))
            .routes(routes!(get_resource_permissions))
            .routes(routes!(update_resource_permissions))
            .routes(routes!(delete_resource_permissions))
            .routes(routes!(get_user_groups))
            .routes(routes!(get_user_effective_permission)),
    )
}

#[utoipa::path(
    post,
    path = "/groups",
    request_body = CreateGroupRequest,
    responses(
        (status = 200, body = CommonResult<GroupResponse>),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 409, description = "Group already exists"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn create_group(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Json(req): Json<CreateGroupRequest>,
) -> Result<Json<CommonResult<GroupResponse>>, ApiError> {
    ensure_admin(&state, &user).await?;

    let name = req.name.trim();
    if name.is_empty() {
        tracing::warn!(
            actor = %user.username,
            "group.create rejected: empty group name"
        );
        return Err(ApiError::bad_request(anyhow!(
            "Group name must not be empty"
        )));
    }

    let description = req
        .description
        .map(|item| item.trim().to_string())
        .and_then(|item| if item.is_empty() { None } else { Some(item) });

    let group = state
        .monorepo()
        .create_group(CreateGroupPayload {
            name: name.to_string(),
            description,
        })
        .await?;

    Ok(Json(CommonResult::success(Some(group.into()))))
}

#[utoipa::path(
    post,
    path = "/groups/list",
    request_body = PageParams<EmptyListAdditional>,
    responses(
        (status = 200, body = CommonResult<CommonPage<GroupResponse>>),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn list_groups(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Json(json): Json<PageParams<EmptyListAdditional>>,
) -> Result<Json<CommonResult<CommonPage<GroupResponse>>>, ApiError> {
    ensure_admin(&state, &user).await?;
    validate_pagination(&json.pagination)?;

    let (items, total) = state.monorepo().list_groups(json.pagination).await?;
    let items = items.into_iter().map(Into::into).collect();

    Ok(Json(CommonResult::success(Some(CommonPage {
        total,
        items,
    }))))
}

#[utoipa::path(
    get,
    path = "/groups/{group_id}",
    params(
        ("group_id" = i64, Path, description = "Group ID")
    ),
    responses(
        (status = 200, body = CommonResult<GroupResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "Group not found"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn get_group(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path(group_id): Path<i64>,
) -> Result<Json<CommonResult<GroupResponse>>, ApiError> {
    ensure_admin(&state, &user).await?;

    let group = state
        .monorepo()
        .get_group_by_id(group_id)
        .await?
        .ok_or_else(|| {
            tracing::warn!(
                actor = %user.username,
                group_id,
                "group.get failed: group not found"
            );
            ApiError::not_found(anyhow!(format!("Group not found: {}", group_id)))
        })?;

    Ok(Json(CommonResult::success(Some(group.into()))))
}

#[utoipa::path(
    delete,
    path = "/groups/{group_id}",
    params(
        ("group_id" = i64, Path, description = "Group ID")
    ),
    responses(
        (status = 200, body = CommonResult<DeleteGroupResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "Group not found"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn delete_group(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path(group_id): Path<i64>,
) -> Result<Json<CommonResult<DeleteGroupResponse>>, ApiError> {
    ensure_admin(&state, &user).await?;

    let stats = state.monorepo().delete_group(group_id).await?;

    Ok(Json(CommonResult::success(Some(DeleteGroupResponse {
        group_id,
        deleted_members_count: stats.deleted_members_count,
        deleted_permissions_count: stats.deleted_permissions_count,
        deleted_groups_count: stats.deleted_groups_count,
    }))))
}

#[utoipa::path(
    post,
    path = "/groups/{group_id}/members",
    request_body = AddMembersRequest,
    params(
        ("group_id" = i64, Path, description = "Group ID")
    ),
    responses(
        (status = 200, body = CommonResult<Vec<GroupMemberResponse>>),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "Group not found"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn add_group_members(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path(group_id): Path<i64>,
    Json(req): Json<AddMembersRequest>,
) -> Result<Json<CommonResult<Vec<GroupMemberResponse>>>, ApiError> {
    ensure_admin(&state, &user).await?;
    if req.usernames.is_empty() {
        tracing::warn!(
            actor = %user.username,
            group_id,
            "group.members.add rejected: empty usernames"
        );
        return Err(ApiError::bad_request(anyhow!(
            "usernames must not be empty"
        )));
    }

    let members = state
        .monorepo()
        .add_group_members(group_id, req.usernames)
        .await?;
    let members = members.into_iter().map(Into::into).collect();

    Ok(Json(CommonResult::success(Some(members))))
}

#[utoipa::path(
    delete,
    path = "/groups/{group_id}/members/{username}",
    params(
        ("group_id" = i64, Path, description = "Group ID"),
        ("username" = String, Path, description = "Username")
    ),
    responses(
        (status = 200, body = CommonResult<RemoveMemberResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "Group not found"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn remove_group_member(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path((group_id, username)): Path<(i64, String)>,
) -> Result<Json<CommonResult<RemoveMemberResponse>>, ApiError> {
    ensure_admin(&state, &user).await?;
    let removed = state
        .monorepo()
        .remove_group_member(group_id, &username)
        .await?;

    Ok(Json(CommonResult::success(Some(RemoveMemberResponse {
        group_id,
        username,
        removed,
    }))))
}

#[utoipa::path(
    post,
    path = "/groups/{group_id}/members/list",
    request_body = PageParams<EmptyListAdditional>,
    params(
        ("group_id" = i64, Path, description = "Group ID")
    ),
    responses(
        (status = 200, body = CommonResult<CommonPage<GroupMemberResponse>>),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "Group not found"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn list_group_members(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path(group_id): Path<i64>,
    Json(json): Json<PageParams<EmptyListAdditional>>,
) -> Result<Json<CommonResult<CommonPage<GroupMemberResponse>>>, ApiError> {
    ensure_admin(&state, &user).await?;
    validate_pagination(&json.pagination)?;

    let (items, total) = state
        .monorepo()
        .list_group_members(group_id, json.pagination)
        .await?;
    let items = items.into_iter().map(Into::into).collect();

    Ok(Json(CommonResult::success(Some(CommonPage {
        total,
        items,
    }))))
}

#[utoipa::path(
    post,
    path = "/resources/{resource_type}/{resource_id}/permissions",
    request_body = SetPermissionsRequest,
    params(
        ("resource_type" = String, Path, description = "Resource type, currently only `note`"),
        ("resource_id" = String, Path, description = "Resource ID")
    ),
    responses(
        (status = 200, body = CommonResult<Vec<ResourcePermissionResponse>>),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "Group not found"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn set_resource_permissions(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path((resource_type, resource_id)): Path<(String, String)>,
    Json(req): Json<SetPermissionsRequest>,
) -> Result<Json<CommonResult<Vec<ResourcePermissionResponse>>>, ApiError> {
    ensure_admin(&state, &user).await?;
    let (resource_type, _, resource_id) =
        parse_resource_context(resource_type.as_str(), &resource_id)?;

    let permissions = req
        .permissions
        .into_iter()
        .map(|item| ResourcePermissionBinding {
            group_id: item.group_id,
            permission: item.permission.into(),
        })
        .collect();

    let saved = state
        .monorepo()
        .set_resource_permission(resource_type, &resource_id, permissions)
        .await?;
    let saved = saved.into_iter().map(Into::into).collect();

    Ok(Json(CommonResult::success(Some(saved))))
}

#[utoipa::path(
    get,
    path = "/resources/{resource_type}/{resource_id}/permissions",
    params(
        ("resource_type" = String, Path, description = "Resource type, currently only `note`"),
        ("resource_id" = String, Path, description = "Resource ID")
    ),
    responses(
        (status = 200, body = CommonResult<Vec<ResourcePermissionResponse>>),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn get_resource_permissions(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path((resource_type, resource_id)): Path<(String, String)>,
) -> Result<Json<CommonResult<Vec<ResourcePermissionResponse>>>, ApiError> {
    ensure_admin(&state, &user).await?;
    let (resource_type, _, resource_id) =
        parse_resource_context(resource_type.as_str(), &resource_id)?;

    let permissions = state
        .monorepo()
        .get_resource_permissions(resource_type, &resource_id)
        .await?;
    let permissions = permissions.into_iter().map(Into::into).collect();

    Ok(Json(CommonResult::success(Some(permissions))))
}

#[utoipa::path(
    put,
    path = "/resources/{resource_type}/{resource_id}/permissions",
    request_body = SetPermissionsRequest,
    params(
        ("resource_type" = String, Path, description = "Resource type, currently only `note`"),
        ("resource_id" = String, Path, description = "Resource ID")
    ),
    responses(
        (status = 200, body = CommonResult<Vec<ResourcePermissionResponse>>),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "Group not found"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn update_resource_permissions(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path((resource_type, resource_id)): Path<(String, String)>,
    Json(req): Json<SetPermissionsRequest>,
) -> Result<Json<CommonResult<Vec<ResourcePermissionResponse>>>, ApiError> {
    ensure_admin(&state, &user).await?;
    let (resource_type, _, resource_id) =
        parse_resource_context(resource_type.as_str(), &resource_id)?;

    let permissions = req
        .permissions
        .into_iter()
        .map(|item| ResourcePermissionBinding {
            group_id: item.group_id,
            permission: item.permission.into(),
        })
        .collect();

    let updated = state
        .monorepo()
        .update_resource_permissions(resource_type, &resource_id, permissions)
        .await?;
    let updated = updated.into_iter().map(Into::into).collect();

    Ok(Json(CommonResult::success(Some(updated))))
}

#[utoipa::path(
    delete,
    path = "/resources/{resource_type}/{resource_id}/permissions",
    params(
        ("resource_type" = String, Path, description = "Resource type, currently only `note`"),
        ("resource_id" = String, Path, description = "Resource ID")
    ),
    responses(
        (status = 200, body = CommonResult<DeletePermissionsResponse>),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn delete_resource_permissions(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path((resource_type, resource_id)): Path<(String, String)>,
) -> Result<Json<CommonResult<DeletePermissionsResponse>>, ApiError> {
    ensure_admin(&state, &user).await?;
    let (resource_type, resource_type_value, resource_id) =
        parse_resource_context(resource_type.as_str(), &resource_id)?;

    let deleted_count = state
        .monorepo()
        .delete_resource_permissions(resource_type, &resource_id)
        .await?;

    Ok(Json(CommonResult::success(Some(
        DeletePermissionsResponse {
            resource_type: resource_type_value,
            resource_id,
            deleted_count,
        },
    ))))
}

#[utoipa::path(
    get,
    path = "/users/{username}/groups",
    params(
        ("username" = String, Path, description = "Username")
    ),
    responses(
        (status = 200, body = CommonResult<UserGroupsResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn get_user_groups(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path(username): Path<String>,
) -> Result<Json<CommonResult<UserGroupsResponse>>, ApiError> {
    ensure_admin(&state, &user).await?;

    let groups = state.monorepo().get_user_groups(&username).await?;
    let groups = groups.into_iter().map(Into::into).collect();

    Ok(Json(CommonResult::success(Some(UserGroupsResponse {
        username,
        groups,
    }))))
}

#[utoipa::path(
    get,
    path = "/users/{username}/permissions/{resource_type}/{resource_id}",
    params(
        ("username" = String, Path, description = "Username"),
        ("resource_type" = String, Path, description = "Resource type, currently only `note`"),
        ("resource_id" = String, Path, description = "Resource ID")
    ),
    responses(
        (status = 200, body = CommonResult<UserEffectivePermissionResponse>),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
    ),
    tag = GROUP_PERMISSION_TAG
)]
async fn get_user_effective_permission(
    user: LoginUser,
    State(state): State<MonoApiServiceState>,
    Path((username, resource_type, resource_id)): Path<(String, String, String)>,
) -> Result<Json<CommonResult<UserEffectivePermissionResponse>>, ApiError> {
    ensure_admin(&state, &user).await?;
    let (resource_type, resource_type_value, resource_id) =
        parse_resource_context(resource_type.as_str(), &resource_id)?;

    let effective = state
        .monorepo()
        .get_user_effective_permission(&username, resource_type, &resource_id)
        .await?;
    let response = build_user_effective_permission_response(
        username,
        resource_type_value,
        resource_id,
        effective,
    );

    Ok(Json(CommonResult::success(Some(response))))
}

fn validate_pagination(pagination: &Pagination) -> Result<(), ApiError> {
    if pagination.page == 0 {
        tracing::warn!("invalid pagination.page: {}", pagination.page);
        return Err(ApiError::bad_request(anyhow!(
            "pagination.page must be >= 1"
        )));
    }
    if pagination.per_page == 0 {
        tracing::warn!("invalid pagination.per_page: {}", pagination.per_page);
        return Err(ApiError::bad_request(anyhow!(
            "pagination.per_page must be >= 1"
        )));
    }
    Ok(())
}
