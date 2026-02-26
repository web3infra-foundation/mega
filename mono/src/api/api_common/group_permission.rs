use anyhow::anyhow;
use callisto::sea_orm_active_enums::ResourceTypeEnum;
use ceres::{
    api_service::group_ops::EffectiveResourcePermission,
    model::group::{PermissionValue, ResourceTypeValue, UserEffectivePermissionResponse},
};
use http::StatusCode;

use crate::api::{MonoApiServiceState, error::ApiError, oauth::model::LoginUser};

pub async fn ensure_admin(state: &MonoApiServiceState, user: &LoginUser) -> Result<(), ApiError> {
    if state.monorepo().check_is_admin(&user.username).await? {
        return Ok(());
    }

    tracing::warn!("admin check failed: access forbidden");

    Err(ApiError::with_status(
        StatusCode::FORBIDDEN,
        anyhow!("Admin access required"),
    ))
}

pub fn parse_resource_context(
    resource_type: &str,
    resource_id: &str,
) -> Result<(ResourceTypeEnum, ResourceTypeValue, String), ApiError> {
    let resource_type_value = ResourceTypeValue::try_from(resource_type).map_err(|err| {
        tracing::warn!("invalid resource_type in request path");
        ApiError::bad_request(anyhow!(err))
    })?;

    let normalized_resource_id = normalize_resource_id(resource_type_value, resource_id)?;

    Ok((
        resource_type_value.into(),
        resource_type_value,
        normalized_resource_id,
    ))
}

pub fn build_user_effective_permission_response(
    username: String,
    resource_type: ResourceTypeValue,
    resource_id: String,
    effective: EffectiveResourcePermission,
) -> UserEffectivePermissionResponse {
    let permission: Option<PermissionValue> = effective.permission.map(Into::into);

    UserEffectivePermissionResponse {
        username,
        resource_type,
        resource_id,
        is_admin: effective.is_admin,
        permission,
        has_read: effective.is_admin || has_permission(permission, PermissionValue::Read),
        has_write: effective.is_admin || has_permission(permission, PermissionValue::Write),
        has_admin: effective.is_admin || has_permission(permission, PermissionValue::Admin),
    }
}

fn normalize_resource_id(
    resource_type: ResourceTypeValue,
    resource_id: &str,
) -> Result<String, ApiError> {
    match resource_type {
        ResourceTypeValue::Note => {
            let note_id = resource_id.parse::<i64>().map_err(|_| {
                tracing::warn!("invalid resource_id format");
                ApiError::bad_request(anyhow!(format!(
                    "Invalid note resource_id: {}, expected i64 note.id",
                    resource_id
                )))
            })?;
            Ok(note_id.to_string())
        }
    }
}

fn has_permission(current: Option<PermissionValue>, required: PermissionValue) -> bool {
    current
        .map(|value| value.satisfies(required))
        .unwrap_or(false)
}
