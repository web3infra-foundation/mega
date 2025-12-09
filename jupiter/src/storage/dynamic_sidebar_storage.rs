use std::ops::Deref;

use crate::{
    model::sidebar_dto::SidebarSyncDto,
    storage::base_storage::{BaseStorage, StorageConnector},
};
use callisto::dynamic_sidebar;
use common::errors::MegaError;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    EntityTrait, QueryOrder, TransactionTrait,
};

#[derive(Clone)]
pub struct DynamicSidebarStorage {
    pub base: BaseStorage,
}

impl Deref for DynamicSidebarStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DynamicSidebarStorage {
    pub async fn get_sidebar_by_id(
        &self,
        id: i32,
    ) -> Result<Option<dynamic_sidebar::Model>, MegaError> {
        let model = dynamic_sidebar::Entity::find_by_id(id)
            .one(self.get_connection())
            .await?;

        Ok(model)
    }

    pub async fn get_sidebars(&self) -> Result<Vec<dynamic_sidebar::Model>, MegaError> {
        let res = dynamic_sidebar::Entity::find()
            .order_by_asc(dynamic_sidebar::Column::OrderIndex)
            .all(self.get_connection())
            .await?;
        Ok(res)
    }

    pub async fn new_sidebar(
        &self,
        public_id: String,
        label: String,
        href: String,
        visible: bool,
        order_index: i32,
    ) -> Result<dynamic_sidebar::Model, MegaError> {
        let active_model = dynamic_sidebar::ActiveModel {
            id: NotSet,
            public_id: Set(public_id),
            label: Set(label),
            href: Set(href),
            visible: Set(visible),
            order_index: Set(order_index),
        };

        let res = active_model.insert(self.get_connection()).await?;
        Ok(res)
    }

    pub async fn update_sidebar(
        &self,
        id: i32,
        public_id: Option<String>,
        label: Option<String>,
        href: Option<String>,
        visible: Option<bool>,
        order_index: Option<i32>,
    ) -> Result<dynamic_sidebar::Model, MegaError> {
        let model = dynamic_sidebar::Entity::find_by_id(id)
            .one(self.get_connection())
            .await?
            .ok_or_else(|| MegaError::Other(format!("Sidebar with id `{id}` not found")))?;

        let mut active_model: dynamic_sidebar::ActiveModel = model.into();

        if let Some(public_id) = public_id {
            active_model.public_id = Set(public_id);
        }
        if let Some(label) = label {
            active_model.label = Set(label);
        }
        if let Some(href) = href {
            active_model.href = Set(href);
        }
        if let Some(visible) = visible {
            active_model.visible = Set(visible);
        }
        if let Some(order_index) = order_index {
            active_model.order_index = Set(order_index);
        }

        let updated_model = active_model.update(self.get_connection()).await?;

        Ok(updated_model)
    }

    pub async fn delete_sidebar(&self, id: i32) -> Result<dynamic_sidebar::Model, MegaError> {
        let model = dynamic_sidebar::Entity::find_by_id(id)
            .one(self.get_connection())
            .await?
            .ok_or_else(|| MegaError::Other(format!("Sidebar with id `{id}` not found")))?;

        let delete_result = dynamic_sidebar::Entity::delete_by_id(id)
            .exec(self.get_connection())
            .await
            .map_err(|e| MegaError::Other(format!("Failed to delete sidebar: {e}")))?;

        if delete_result.rows_affected == 0 {
            return Err(MegaError::Other(format!(
                "Sidebar with id `{id}` was not deleted"
            )));
        }

        Ok(model)
    }

    pub async fn sync_sidebar(
        &self,
        items: Vec<SidebarSyncDto>,
    ) -> Result<Vec<dynamic_sidebar::Model>, MegaError> {
        // Begin a transaction
        let txn = self.get_connection().begin().await?;

        let mut res_models = Vec::with_capacity(items.len());

        for item in items {
            if let Some(id) = item.id {
                // Update existing menu item
                if let Some(model) = dynamic_sidebar::Entity::find_by_id(id).one(&txn).await? {
                    let mut active_model: dynamic_sidebar::ActiveModel = model.into();

                    active_model.public_id = Set(item.public_id);
                    active_model.label = Set(item.label);
                    active_model.href = Set(item.href);
                    active_model.visible = Set(item.visible);
                    active_model.order_index = Set(item.order_index);

                    let updated = active_model.update(&txn).await?;
                    res_models.push(updated);
                } else {
                    return Err(MegaError::Other(format!(
                        "Sidebar with id `{id}` not found"
                    )));
                }
            } else {
                // Insert new menu item
                let active_model = dynamic_sidebar::ActiveModel {
                    id: NotSet,
                    public_id: Set(item.public_id),
                    label: Set(item.label),
                    href: Set(item.href),
                    visible: Set(item.visible),
                    order_index: Set(item.order_index),
                };
                let inserted = active_model.insert(&txn).await?;
                res_models.push(inserted);
            }
        }

        // Commit the transaction
        txn.commit().await?;

        Ok(res_models)
    }
}
