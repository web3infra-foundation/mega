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

    tracing::warn!(
        actor = %user.username,
        "admin check failed: access forbidden"
    );

    Err(ApiError::with_status(
        StatusCode::FORBIDDEN,
        anyhow!("Admin access required"),
    ))
}

pub async fn resolve_resource_context(
    state: &MonoApiServiceState,
    resource_type: &str,
    resource_id: &str,
) -> Result<(ResourceTypeEnum, ResourceTypeValue, String), ApiError> {
    let resource_type_value = ResourceTypeValue::try_from(resource_type).map_err(|err| {
        tracing::warn!("invalid resource_type in request path");
        ApiError::bad_request(anyhow!(err))
    })?;

    let validated_resource_id =
        resolve_resource_id(state, resource_type_value, resource_id).await?;

    Ok((
        resource_type_value.into(),
        resource_type_value,
        validated_resource_id,
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

async fn resolve_resource_id(
    state: &MonoApiServiceState,
    resource_type: ResourceTypeValue,
    resource_id: &str,
) -> Result<String, ApiError> {
    let normalized_resource_id = resource_id.trim();
    if normalized_resource_id.is_empty() {
        tracing::warn!("empty resource_id in request path");
        return Err(ApiError::bad_request(anyhow!(
            "resource_id must not be empty"
        )));
    }

    match resource_type {
        ResourceTypeValue::Note => {
            let note = state
                .note_stg()
                .get_note_by_public_id(normalized_resource_id)
                .await?
                .ok_or_else(|| {
                    tracing::warn!(
                        resource_id = normalized_resource_id,
                        "note resource not found"
                    );
                    ApiError::not_found(anyhow!(
                        "Note not found for public_id: {}",
                        normalized_resource_id
                    ))
                })?;
            Ok(note.public_id)
        }
    }
}

fn has_permission(current: Option<PermissionValue>, required: PermissionValue) -> bool {
    current
        .map(|value| value.satisfies(required))
        .unwrap_or(false)
}
