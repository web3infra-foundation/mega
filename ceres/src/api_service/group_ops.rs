use api_model::common::Pagination;
use callisto::{
    mega_group, mega_group_member, mega_resource_permission,
    sea_orm_active_enums::{PermissionEnum, ResourceTypeEnum},
};
use common::errors::MegaError;
use jupiter::model::group_dto::{CreateGroupPayload, DeleteGroupStats, ResourcePermissionBinding};

use crate::api_service::mono_api_service::MonoApiService;

#[derive(Debug, Clone)]
pub struct EffectiveResourcePermission {
    pub is_admin: bool,
    pub permission: Option<PermissionEnum>,
}

impl MonoApiService {
    pub async fn create_group(
        &self,
        payload: CreateGroupPayload,
    ) -> Result<mega_group::Model, MegaError> {
        self.storage.group_storage().create_group(payload).await
    }

    pub async fn list_groups(
        &self,
        page: Pagination,
    ) -> Result<(Vec<mega_group::Model>, u64), MegaError> {
        self.storage.group_storage().list_groups(page).await
    }

    pub async fn get_group_by_id(
        &self,
        group_id: i64,
    ) -> Result<Option<mega_group::Model>, MegaError> {
        self.storage.group_storage().get_group_by_id(group_id).await
    }

    pub async fn delete_group(&self, group_id: i64) -> Result<DeleteGroupStats, MegaError> {
        let stats = self
            .storage
            .group_storage()
            .delete_group_with_relations(group_id)
            .await?;

        if stats.deleted_groups_count == 0 {
            return Err(MegaError::NotFound(format!(
                "Group not found: {}",
                group_id
            )));
        }

        Ok(stats)
    }

    pub async fn add_group_members(
        &self,
        group_id: i64,
        usernames: Vec<String>,
    ) -> Result<Vec<mega_group_member::Model>, MegaError> {
        let group_storage = self.storage.group_storage();
        group_storage.add_group_members(group_id, &usernames).await
    }

    pub async fn remove_group_member(
        &self,
        group_id: i64,
        username: &str,
    ) -> Result<bool, MegaError> {
        let group_storage = self.storage.group_storage();
        if group_storage.get_group_by_id(group_id).await?.is_none() {
            return Err(MegaError::NotFound(format!(
                "Group not found: {}",
                group_id
            )));
        }
        group_storage.remove_group_member(group_id, username).await
    }

    pub async fn list_group_members(
        &self,
        group_id: i64,
        page: Pagination,
    ) -> Result<(Vec<mega_group_member::Model>, u64), MegaError> {
        let group_storage = self.storage.group_storage();
        if group_storage.get_group_by_id(group_id).await?.is_none() {
            return Err(MegaError::NotFound(format!(
                "Group not found: {}",
                group_id
            )));
        }
        group_storage.list_group_members(group_id, page).await
    }

    pub async fn set_resource_permission(
        &self,
        resource_type: ResourceTypeEnum,
        resource_id: &str,
        permissions: Vec<ResourcePermissionBinding>,
    ) -> Result<Vec<mega_resource_permission::Model>, MegaError> {
        self.storage
            .group_storage()
            .replace_resource_permissions(resource_type, resource_id, &permissions)
            .await
    }

    pub async fn get_resource_permissions(
        &self,
        resource_type: ResourceTypeEnum,
        resource_id: &str,
    ) -> Result<Vec<mega_resource_permission::Model>, MegaError> {
        self.storage
            .group_storage()
            .list_resource_permissions(resource_type, resource_id)
            .await
    }

    pub async fn update_resource_permissions(
        &self,
        resource_type: ResourceTypeEnum,
        resource_id: &str,
        permissions: Vec<ResourcePermissionBinding>,
    ) -> Result<Vec<mega_resource_permission::Model>, MegaError> {
        self.storage
            .group_storage()
            .upsert_resource_permissions(resource_type, resource_id, &permissions)
            .await
    }

    pub async fn delete_resource_permissions(
        &self,
        resource_type: ResourceTypeEnum,
        resource_id: &str,
    ) -> Result<u64, MegaError> {
        self.storage
            .group_storage()
            .delete_resource_permissions(resource_type, resource_id)
            .await
    }

    pub async fn get_user_groups(
        &self,
        username: &str,
    ) -> Result<Vec<mega_group::Model>, MegaError> {
        self.storage
            .group_storage()
            .find_groups_by_username(username)
            .await
    }

    pub async fn get_user_effective_permission(
        &self,
        username: &str,
        resource_type: ResourceTypeEnum,
        resource_id: &str,
    ) -> Result<EffectiveResourcePermission, MegaError> {
        if self.check_is_admin(username).await? {
            return Ok(EffectiveResourcePermission {
                is_admin: true,
                permission: Some(PermissionEnum::Admin),
            });
        }

        let group_storage = self.storage.group_storage();
        let group_ids = group_storage
            .find_group_ids_by_username(username)
            .await
            .map_err(|_| MegaError::Other("Failed to query user group memberships".to_string()))?;
        if group_ids.is_empty() {
            return Ok(EffectiveResourcePermission {
                is_admin: false,
                permission: None,
            });
        }

        let permissions = group_storage
            .find_permissions_by_resource(resource_type, resource_id, &group_ids)
            .await
            .map_err(|_| MegaError::Other("Failed to query resource permissions".to_string()))?;

        let permission = permissions
            .iter()
            .map(|item| item.permission.clone())
            .max_by_key(permission_level);

        Ok(EffectiveResourcePermission {
            is_admin: false,
            permission,
        })
    }

    /// Reserved for future business-route authorization integration.
    pub async fn check_resource_permission(
        &self,
        username: &str,
        resource_type: ResourceTypeEnum,
        resource_id: &str,
        required_permission: PermissionEnum,
    ) -> Result<bool, MegaError> {
        let effective = self
            .get_user_effective_permission(username, resource_type, resource_id)
            .await?;

        if effective.is_admin {
            return Ok(true);
        }

        Ok(match effective.permission {
            Some(permission) => permission_satisfies(&permission, &required_permission),
            None => false,
        })
    }
}

fn permission_level(permission: &PermissionEnum) -> u8 {
    match permission {
        PermissionEnum::Read => 1,
        PermissionEnum::Write => 2,
        PermissionEnum::Admin => 3,
    }
}

fn permission_satisfies(current: &PermissionEnum, required: &PermissionEnum) -> bool {
    permission_level(current) >= permission_level(required)
}
