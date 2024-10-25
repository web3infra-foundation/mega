use std::sync::Arc;

use callisto::{
    ztm_lfs_info, ztm_node, ztm_nostr_event, ztm_nostr_req, ztm_path_mapping, ztm_repo_info,
};
use common::errors::MegaError;
use sea_orm::InsertResult;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, Set};

#[derive(Clone)]
pub struct ZTMStorage {
    pub connection: Arc<DatabaseConnection>,
}

impl ZTMStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>) -> Self {
        ZTMStorage { connection }
    }

    pub fn mock() -> Self {
        ZTMStorage {
            connection: Arc::new(DatabaseConnection::default()),
        }
    }

    pub async fn get_node_by_id(
        &self,
        peer_id: &str,
    ) -> Result<Option<ztm_node::Model>, MegaError> {
        let result = ztm_node::Entity::find_by_id(peer_id)
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(result)
    }

    pub async fn get_all_node(&self) -> Result<Vec<ztm_node::Model>, MegaError> {
        Ok(ztm_node::Entity::find()
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn insert_node(
        &self,
        node: ztm_node::Model,
    ) -> Result<InsertResult<ztm_node::ActiveModel>, MegaError> {
        Ok(ztm_node::Entity::insert(node.into_active_model())
            .exec(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn update_node(&self, node: ztm_node::Model) -> Result<ztm_node::Model, MegaError> {
        let mut active_model: ztm_node::ActiveModel = node.clone().into_active_model();
        active_model.hub = Set(node.hub);
        active_model.agent_name = Set(node.agent_name);
        active_model.service_name = Set(node.service_name);
        active_model.r#type = Set(node.r#type);
        active_model.online = Set(node.online);
        active_model.last_online_time = Set(node.last_online_time);
        Ok(ztm_node::Entity::update(active_model)
            .exec(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn insert_or_update_node(
        &self,
        node: ztm_node::Model,
    ) -> Result<ztm_node::Model, MegaError> {
        match self.get_node_by_id(&node.peer_id).await.unwrap() {
            Some(_) => {
                let mut active_model: ztm_node::ActiveModel = node.clone().into_active_model();
                active_model.hub = Set(node.hub);
                active_model.agent_name = Set(node.agent_name);
                active_model.service_name = Set(node.service_name);
                active_model.r#type = Set(node.r#type);
                active_model.online = Set(node.online);
                active_model.last_online_time = Set(node.last_online_time);
                Ok(ztm_node::Entity::update(active_model)
                    .exec(self.get_connection())
                    .await
                    .unwrap())
            }
            None => {
                ztm_node::Entity::insert(node.clone().into_active_model())
                    .exec(self.get_connection())
                    .await
                    .unwrap();
                Ok(node)
            }
        }
    }

    pub async fn delete_node_by_id(&self, id: String) {
        ztm_node::Entity::delete_by_id(id)
            .exec(self.get_connection())
            .await
            .unwrap();
    }

    pub async fn get_repo_info_by_id(
        &self,
        identifier: &str,
    ) -> Result<Option<ztm_repo_info::Model>, MegaError> {
        let result = ztm_repo_info::Entity::find_by_id(identifier)
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(result)
    }

    pub async fn get_all_repo_info(&self) -> Result<Vec<ztm_repo_info::Model>, MegaError> {
        Ok(ztm_repo_info::Entity::find()
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn insert_repo_info(
        &self,
        repo_info: ztm_repo_info::Model,
    ) -> Result<InsertResult<ztm_repo_info::ActiveModel>, MegaError> {
        Ok(ztm_repo_info::Entity::insert(repo_info.into_active_model())
            .exec(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn update_repo_info(
        &self,
        repo_info: ztm_repo_info::Model,
    ) -> Result<ztm_repo_info::Model, MegaError> {
        Ok(ztm_repo_info::Entity::update(repo_info.into_active_model())
            .exec(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn insert_or_update_repo_info(
        &self,
        repo_info: ztm_repo_info::Model,
    ) -> Result<ztm_repo_info::Model, MegaError> {
        match self
            .get_repo_info_by_id(&repo_info.identifier)
            .await
            .unwrap()
        {
            Some(_) => {
                let mut active_model: ztm_repo_info::ActiveModel =
                    repo_info.clone().into_active_model();
                active_model.name = Set(repo_info.name);
                active_model.origin = Set(repo_info.origin);
                active_model.update_time = Set(repo_info.update_time);
                active_model.commit = Set(repo_info.commit);
                Ok(ztm_repo_info::Entity::update(active_model)
                    .exec(self.get_connection())
                    .await
                    .unwrap())
            }
            None => {
                ztm_repo_info::Entity::insert(repo_info.clone().into_active_model())
                    .exec(self.get_connection())
                    .await
                    .unwrap();
                Ok(repo_info)
            }
        }
    }

    pub async fn insert_lfs_info(
        &self,
        lfs_info: ztm_lfs_info::Model,
    ) -> Result<ztm_lfs_info::Model, MegaError> {
        let list = self
            .get_lfs_info_by_origin_and_peerid(&lfs_info.origin, &lfs_info.peer_id)
            .await
            .unwrap();
        if list.is_empty() {
            ztm_lfs_info::Entity::insert(lfs_info.clone().into_active_model())
                .exec(self.get_connection())
                .await
                .unwrap();
        }
        Ok(lfs_info)
    }

    pub async fn get_lfs_info_by_origin_and_peerid(
        &self,
        origin: &str,
        peer_id: &str,
    ) -> Result<Vec<ztm_lfs_info::Model>, MegaError> {
        let model = ztm_lfs_info::Entity::find()
            .filter(ztm_lfs_info::Column::Origin.eq(origin))
            .filter(ztm_lfs_info::Column::PeerId.eq(peer_id))
            .all(self.get_connection())
            .await;
        Ok(model?)
    }

    pub async fn get_all_lfs_info(&self) -> Result<Vec<ztm_lfs_info::Model>, MegaError> {
        Ok(ztm_lfs_info::Entity::find()
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn insert_nostr_event(
        &self,
        nostr_event: ztm_nostr_event::Model,
    ) -> Result<InsertResult<ztm_nostr_event::ActiveModel>, MegaError> {
        Ok(
            ztm_nostr_event::Entity::insert(nostr_event.into_active_model())
                .exec(self.get_connection())
                .await
                .unwrap(),
        )
    }

    pub async fn get_nostr_event_by_id(
        &self,
        event_id: &str,
    ) -> Result<Option<ztm_nostr_event::Model>, MegaError> {
        let result = ztm_nostr_event::Entity::find_by_id(event_id)
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(result)
    }

    pub async fn get_all_nostr_event(&self) -> Result<Vec<ztm_nostr_event::Model>, MegaError> {
        Ok(ztm_nostr_event::Entity::find()
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_all_nostr_event_by_pubkey(
        &self,
        pubkey: &str,
    ) -> Result<Vec<ztm_nostr_event::Model>, MegaError> {
        Ok(ztm_nostr_event::Entity::find()
            .filter(ztm_nostr_event::Column::Pubkey.eq(pubkey))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn insert_nostr_req(
        &self,
        nostr_req: ztm_nostr_req::Model,
    ) -> Result<InsertResult<ztm_nostr_req::ActiveModel>, MegaError> {
        Ok(ztm_nostr_req::Entity::insert(nostr_req.into_active_model())
            .exec(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_all_nostr_req_by_subscription_id(
        &self,
        subscription_id: &str,
    ) -> Result<Vec<ztm_nostr_req::Model>, MegaError> {
        Ok(ztm_nostr_req::Entity::find()
            .filter(ztm_nostr_req::Column::SubscriptionId.eq(subscription_id))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_all_nostr_req(&self) -> Result<Vec<ztm_nostr_req::Model>, MegaError> {
        Ok(ztm_nostr_req::Entity::find()
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn save_alias_mapping(
        &self,
        model: ztm_path_mapping::Model,
    ) -> Result<(), MegaError> {
        ztm_path_mapping::Entity::insert(model.into_active_model())
            .exec(self.get_connection())
            .await
            .map_err(|err| {
                tracing::error!("Error saving alias mapping: {}", err);
                err
            })?;
        Ok(())
    }

    pub async fn get_path_from_alias(
        &self,
        alias: &str,
    ) -> Result<Option<ztm_path_mapping::Model>, MegaError> {
        Ok(ztm_path_mapping::Entity::find()
            .filter(ztm_path_mapping::Column::Alias.eq(alias))
            .one(self.get_connection())
            .await
            .unwrap())
    }
}
