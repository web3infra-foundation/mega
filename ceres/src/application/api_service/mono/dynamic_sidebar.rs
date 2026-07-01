use common::errors::MegaError;

use super::context::SidebarApplicationService;
use crate::model::dynamic_sidebar::{SidebarMenuListRes, SidebarRes, SidebarSyncPayload};

impl SidebarApplicationService {
    pub async fn list_sidebars(&self) -> Result<SidebarMenuListRes, MegaError> {
        Ok(self
            .ctx
            .storage()
            .dynamic_sidebar_storage()
            .get_sidebars()
            .await?
            .into_iter()
            .map(|m| m.into())
            .collect())
    }

    pub async fn new_sidebar(
        &self,
        public_id: String,
        label: String,
        href: String,
        visible: bool,
        order_index: i32,
    ) -> Result<SidebarRes, MegaError> {
        let res = self
            .ctx
            .storage()
            .dynamic_sidebar_storage()
            .new_sidebar(public_id, label, href, visible, order_index)
            .await?;
        Ok(res.into())
    }

    pub async fn update_sidebar(
        &self,
        id: i32,
        public_id: Option<String>,
        label: Option<String>,
        href: Option<String>,
        visible: Option<bool>,
        order_index: Option<i32>,
    ) -> Result<SidebarRes, MegaError> {
        let res = self
            .ctx
            .storage()
            .dynamic_sidebar_storage()
            .update_sidebar(id, public_id, label, href, visible, order_index)
            .await?;
        Ok(res.into())
    }

    pub async fn sync_sidebars(
        &self,
        payloads: Vec<SidebarSyncPayload>,
    ) -> Result<Vec<SidebarRes>, MegaError> {
        let res = self
            .ctx
            .storage()
            .dynamic_sidebar_storage()
            .sync_sidebar(payloads.into_iter().map(|item| item.into()).collect())
            .await?;
        Ok(res.into_iter().map(|item| item.into()).collect())
    }

    pub async fn delete_sidebar(&self, id: i32) -> Result<SidebarRes, MegaError> {
        let res = self
            .ctx
            .storage()
            .dynamic_sidebar_storage()
            .delete_sidebar(id)
            .await?;
        Ok(res.into())
    }
}
