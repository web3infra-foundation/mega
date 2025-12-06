use std::ops::Deref;

use crate::storage::base_storage::{BaseStorage, StorageConnector};
use callisto::dynamic_sidebar;
use common::errors::MegaError;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    EntityTrait,
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
            .ok_or_else(|| MegaError::Other(format!("Sidebar with Id `{id}` not found")))?;

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
}
