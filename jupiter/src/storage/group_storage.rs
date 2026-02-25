use std::{
    collections::{BTreeMap, HashSet},
    ops::Deref,
};

use api_model::common::Pagination;
use callisto::{
    mega_group, mega_group_member, mega_resource_permission, sea_orm_active_enums::ResourceTypeEnum,
};
use common::{errors::MegaError, utils::generate_id};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DbErr, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait, sea_query::OnConflict,
};

use crate::{
    model::group_dto::{CreateGroupPayload, DeleteGroupStats, ResourcePermissionBinding},
    storage::base_storage::{BaseStorage, StorageConnector},
};

#[derive(Clone, Debug)]
pub struct GroupStorage {
    pub base: BaseStorage,
}

impl Deref for GroupStorage {
    type Target = BaseStorage;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl GroupStorage {
    pub async fn create_group(
        &self,
        payload: CreateGroupPayload,
    ) -> Result<mega_group::Model, MegaError> {
        let now = chrono::Utc::now().naive_utc();
        let group = mega_group::ActiveModel {
            id: Set(generate_id()),
            name: Set(payload.name),
            description: Set(payload.description),
            created_at: Set(now),
            updated_at: Set(now),
        };

        Ok(group.insert(self.get_connection()).await?)
    }

    pub async fn list_groups(
        &self,
        page: Pagination,
    ) -> Result<(Vec<mega_group::Model>, u64), MegaError> {
        let paginator = mega_group::Entity::find()
            .order_by_desc(mega_group::Column::CreatedAt)
            .paginate(self.get_connection(), page.per_page);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.page.saturating_sub(1)).await?;

        Ok((items, total))
    }

    pub async fn get_group_by_id(
        &self,
        group_id: i64,
    ) -> Result<Option<mega_group::Model>, MegaError> {
        Ok(mega_group::Entity::find_by_id(group_id)
            .one(self.get_connection())
            .await?)
    }

    pub async fn get_group_by_name(
        &self,
        name: &str,
    ) -> Result<Option<mega_group::Model>, MegaError> {
        Ok(mega_group::Entity::find()
            .filter(mega_group::Column::Name.eq(name))
            .one(self.get_connection())
            .await?)
    }

    pub async fn delete_group_with_relations(
        &self,
        group_id: i64,
    ) -> Result<DeleteGroupStats, MegaError> {
        let txn = self.get_connection().begin().await?;

        let deleted_members_count = mega_group_member::Entity::delete_many()
            .filter(mega_group_member::Column::GroupId.eq(group_id))
            .exec(&txn)
            .await?
            .rows_affected;

        let deleted_permissions_count = mega_resource_permission::Entity::delete_many()
            .filter(mega_resource_permission::Column::GroupId.eq(group_id))
            .exec(&txn)
            .await?
            .rows_affected;

        let deleted_groups_count = mega_group::Entity::delete_many()
            .filter(mega_group::Column::Id.eq(group_id))
            .exec(&txn)
            .await?
            .rows_affected;

        txn.commit().await?;

        Ok(DeleteGroupStats {
            deleted_members_count,
            deleted_permissions_count,
            deleted_groups_count,
        })
    }

    pub async fn add_group_members(
        &self,
        group_id: i64,
        usernames: &[String],
    ) -> Result<Vec<mega_group_member::Model>, MegaError> {
        let usernames = normalize_usernames(usernames);
        if usernames.is_empty() {
            return Ok(Vec::new());
        }

        let now = chrono::Utc::now().naive_utc();
        let models = usernames
            .iter()
            .map(|username| mega_group_member::ActiveModel {
                id: Set(generate_id()),
                group_id: Set(group_id),
                username: Set(username.clone()),
                joined_at: Set(now),
            })
            .collect::<Vec<_>>();

        let on_conflict = OnConflict::columns([
            mega_group_member::Column::GroupId,
            mega_group_member::Column::Username,
        ])
        .do_nothing()
        .to_owned();

        match mega_group_member::Entity::insert_many(models)
            .on_conflict(on_conflict)
            .exec(self.get_connection())
            .await
        {
            Ok(_) | Err(DbErr::RecordNotInserted) => {}
            Err(e) => return Err(e.into()),
        }

        Ok(mega_group_member::Entity::find()
            .filter(mega_group_member::Column::GroupId.eq(group_id))
            .filter(mega_group_member::Column::Username.is_in(usernames))
            .order_by_asc(mega_group_member::Column::JoinedAt)
            .all(self.get_connection())
            .await?)
    }

    pub async fn remove_group_member(
        &self,
        group_id: i64,
        username: &str,
    ) -> Result<bool, MegaError> {
        let result = mega_group_member::Entity::delete_many()
            .filter(mega_group_member::Column::GroupId.eq(group_id))
            .filter(mega_group_member::Column::Username.eq(username))
            .exec(self.get_connection())
            .await?;

        Ok(result.rows_affected > 0)
    }

    pub async fn list_group_members(
        &self,
        group_id: i64,
        page: Pagination,
    ) -> Result<(Vec<mega_group_member::Model>, u64), MegaError> {
        let paginator = mega_group_member::Entity::find()
            .filter(mega_group_member::Column::GroupId.eq(group_id))
            .order_by_asc(mega_group_member::Column::JoinedAt)
            .paginate(self.get_connection(), page.per_page);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.page.saturating_sub(1)).await?;

        Ok((items, total))
    }

    pub async fn find_group_ids_by_username(&self, username: &str) -> Result<Vec<i64>, MegaError> {
        Ok(mega_group_member::Entity::find()
            .select_only()
            .column(mega_group_member::Column::GroupId)
            .filter(mega_group_member::Column::Username.eq(username))
            .into_tuple::<i64>()
            .all(self.get_connection())
            .await?)
    }

    pub async fn find_groups_by_username(
        &self,
        username: &str,
    ) -> Result<Vec<mega_group::Model>, MegaError> {
        let group_ids = self.find_group_ids_by_username(username).await?;
        if group_ids.is_empty() {
            return Ok(Vec::new());
        }

        Ok(mega_group::Entity::find()
            .filter(mega_group::Column::Id.is_in(group_ids))
            .order_by_asc(mega_group::Column::Name)
            .all(self.get_connection())
            .await?)
    }

    pub async fn list_resource_permissions(
        &self,
        resource_type: ResourceTypeEnum,
        resource_id: &str,
    ) -> Result<Vec<mega_resource_permission::Model>, MegaError> {
        Ok(mega_resource_permission::Entity::find()
            .filter(mega_resource_permission::Column::ResourceType.eq(resource_type))
            .filter(mega_resource_permission::Column::ResourceId.eq(resource_id))
            .order_by_asc(mega_resource_permission::Column::GroupId)
            .all(self.get_connection())
            .await?)
    }

    pub async fn replace_resource_permissions(
        &self,
        resource_type: ResourceTypeEnum,
        resource_id: &str,
        permissions: &[ResourcePermissionBinding],
    ) -> Result<Vec<mega_resource_permission::Model>, MegaError> {
        let permissions = normalize_permission_bindings(permissions);
        let txn = self.get_connection().begin().await?;

        mega_resource_permission::Entity::delete_many()
            .filter(mega_resource_permission::Column::ResourceType.eq(resource_type.clone()))
            .filter(mega_resource_permission::Column::ResourceId.eq(resource_id))
            .exec(&txn)
            .await?;

        if !permissions.is_empty() {
            let now = chrono::Utc::now().naive_utc();
            let models = permissions
                .iter()
                .map(|binding| mega_resource_permission::ActiveModel {
                    id: Set(generate_id()),
                    resource_type: Set(resource_type.clone()),
                    resource_id: Set(resource_id.to_string()),
                    group_id: Set(binding.group_id),
                    permission: Set(binding.permission.clone()),
                    created_at: Set(now),
                    updated_at: Set(now),
                })
                .collect::<Vec<_>>();

            mega_resource_permission::Entity::insert_many(models)
                .exec(&txn)
                .await?;
        }

        let result = mega_resource_permission::Entity::find()
            .filter(mega_resource_permission::Column::ResourceType.eq(resource_type.clone()))
            .filter(mega_resource_permission::Column::ResourceId.eq(resource_id))
            .order_by_asc(mega_resource_permission::Column::GroupId)
            .all(&txn)
            .await?;

        txn.commit().await?;
        Ok(result)
    }

    pub async fn upsert_resource_permissions(
        &self,
        resource_type: ResourceTypeEnum,
        resource_id: &str,
        permissions: &[ResourcePermissionBinding],
    ) -> Result<Vec<mega_resource_permission::Model>, MegaError> {
        let permissions = normalize_permission_bindings(permissions);
        if permissions.is_empty() {
            return self
                .list_resource_permissions(resource_type, resource_id)
                .await;
        }

        let now = chrono::Utc::now().naive_utc();
        let models = permissions
            .iter()
            .map(|binding| mega_resource_permission::ActiveModel {
                id: Set(generate_id()),
                resource_type: Set(resource_type.clone()),
                resource_id: Set(resource_id.to_string()),
                group_id: Set(binding.group_id),
                permission: Set(binding.permission.clone()),
                created_at: Set(now),
                updated_at: Set(now),
            })
            .collect::<Vec<_>>();

        let on_conflict = OnConflict::columns([
            mega_resource_permission::Column::ResourceType,
            mega_resource_permission::Column::ResourceId,
            mega_resource_permission::Column::GroupId,
        ])
        .update_columns([
            mega_resource_permission::Column::Permission,
            mega_resource_permission::Column::UpdatedAt,
        ])
        .to_owned();

        match mega_resource_permission::Entity::insert_many(models)
            .on_conflict(on_conflict)
            .exec(self.get_connection())
            .await
        {
            Ok(_) | Err(DbErr::RecordNotInserted) => {}
            Err(e) => return Err(e.into()),
        }

        self.list_resource_permissions(resource_type, resource_id)
            .await
    }

    pub async fn delete_resource_permissions(
        &self,
        resource_type: ResourceTypeEnum,
        resource_id: &str,
    ) -> Result<u64, MegaError> {
        let result = mega_resource_permission::Entity::delete_many()
            .filter(mega_resource_permission::Column::ResourceType.eq(resource_type))
            .filter(mega_resource_permission::Column::ResourceId.eq(resource_id))
            .exec(self.get_connection())
            .await?;

        Ok(result.rows_affected)
    }

    pub async fn delete_resource_permissions_by_group_id(
        &self,
        group_id: i64,
    ) -> Result<u64, MegaError> {
        let result = mega_resource_permission::Entity::delete_many()
            .filter(mega_resource_permission::Column::GroupId.eq(group_id))
            .exec(self.get_connection())
            .await?;

        Ok(result.rows_affected)
    }

    pub async fn find_permissions_by_resource(
        &self,
        resource_type: ResourceTypeEnum,
        resource_id: &str,
        group_ids: &[i64],
    ) -> Result<Vec<mega_resource_permission::Model>, MegaError> {
        if group_ids.is_empty() {
            return Ok(Vec::new());
        }

        Ok(mega_resource_permission::Entity::find()
            .filter(mega_resource_permission::Column::ResourceType.eq(resource_type))
            .filter(mega_resource_permission::Column::ResourceId.eq(resource_id))
            .filter(mega_resource_permission::Column::GroupId.is_in(group_ids.to_vec()))
            .order_by_asc(mega_resource_permission::Column::GroupId)
            .all(self.get_connection())
            .await?)
    }
}

fn normalize_usernames(usernames: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();

    usernames
        .iter()
        .map(|username| username.trim())
        .filter(|username| !username.is_empty())
        .filter_map(|username| {
            let s = username.to_string();
            if seen.insert(s.clone()) {
                Some(s)
            } else {
                None
            }
        })
        .collect()
}

fn normalize_permission_bindings(
    permissions: &[ResourcePermissionBinding],
) -> Vec<ResourcePermissionBinding> {
    let mut by_group = BTreeMap::new();
    for permission in permissions {
        by_group.insert(permission.group_id, permission.permission.clone());
    }

    by_group
        .into_iter()
        .map(|(group_id, permission)| ResourcePermissionBinding {
            group_id,
            permission,
        })
        .collect()
}
