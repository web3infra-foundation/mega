use std::sync::Arc;

use callisto::{
    relay_lfs_info, relay_node, relay_nostr_event, relay_nostr_req, relay_path_mapping,
    relay_repo_info,
};
use common::errors::MegaError;
use sea_orm::InsertResult;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, Set};

#[derive(Clone)]
pub struct RelayStorage {
    pub connection: Arc<DatabaseConnection>,
}

impl RelayStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>) -> Self {
        RelayStorage { connection }
    }

    pub fn mock() -> Self {
        RelayStorage {
            connection: Arc::new(DatabaseConnection::default()),
        }
    }

    pub async fn get_node_by_id(
        &self,
        peer_id: &str,
    ) -> Result<Option<relay_node::Model>, MegaError> {
        let result = relay_node::Entity::find_by_id(peer_id)
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(result)
    }

    pub async fn get_all_node(&self) -> Result<Vec<relay_node::Model>, MegaError> {
        Ok(relay_node::Entity::find()
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn insert_node(
        &self,
        node: relay_node::Model,
    ) -> Result<InsertResult<relay_node::ActiveModel>, MegaError> {
        Ok(relay_node::Entity::insert(node.into_active_model())
            .exec(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn update_node(
        &self,
        node: relay_node::Model,
    ) -> Result<relay_node::Model, MegaError> {
        let mut active_model: relay_node::ActiveModel = node.clone().into_active_model();
        active_model.r#type = Set(node.r#type);
        active_model.online = Set(node.online);
        active_model.last_online_time = Set(node.last_online_time);
        Ok(relay_node::Entity::update(active_model)
            .exec(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn insert_or_update_node(
        &self,
        node: relay_node::Model,
    ) -> Result<relay_node::Model, MegaError> {
        match self.get_node_by_id(&node.peer_id).await.unwrap() {
            Some(_) => {
                let mut active_model: relay_node::ActiveModel = node.clone().into_active_model();
                active_model.r#type = Set(node.r#type);
                active_model.online = Set(node.online);
                active_model.last_online_time = Set(node.last_online_time);
                Ok(relay_node::Entity::update(active_model)
                    .exec(self.get_connection())
                    .await
                    .unwrap())
            }
            None => {
                relay_node::Entity::insert(node.clone().into_active_model())
                    .exec(self.get_connection())
                    .await
                    .unwrap();
                Ok(node)
            }
        }
    }

    pub async fn delete_node_by_id(&self, id: String) {
        relay_node::Entity::delete_by_id(id)
            .exec(self.get_connection())
            .await
            .unwrap();
    }

    pub async fn get_repo_info_by_id(
        &self,
        identifier: &str,
    ) -> Result<Option<relay_repo_info::Model>, MegaError> {
        let result = relay_repo_info::Entity::find_by_id(identifier)
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(result)
    }

    pub async fn get_all_repo_info(&self) -> Result<Vec<relay_repo_info::Model>, MegaError> {
        Ok(relay_repo_info::Entity::find()
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn insert_repo_info(
        &self,
        repo_info: relay_repo_info::Model,
    ) -> Result<InsertResult<relay_repo_info::ActiveModel>, MegaError> {
        Ok(
            relay_repo_info::Entity::insert(repo_info.into_active_model())
                .exec(self.get_connection())
                .await
                .unwrap(),
        )
    }

    pub async fn update_repo_info(
        &self,
        repo_info: relay_repo_info::Model,
    ) -> Result<relay_repo_info::Model, MegaError> {
        Ok(
            relay_repo_info::Entity::update(repo_info.into_active_model())
                .exec(self.get_connection())
                .await
                .unwrap(),
        )
    }

    pub async fn insert_or_update_repo_info(
        &self,
        repo_info: relay_repo_info::Model,
    ) -> Result<relay_repo_info::Model, MegaError> {
        match self
            .get_repo_info_by_id(&repo_info.identifier)
            .await
            .unwrap()
        {
            Some(_) => {
                let mut active_model: relay_repo_info::ActiveModel =
                    repo_info.clone().into_active_model();
                active_model.name = Set(repo_info.name);
                active_model.origin = Set(repo_info.origin);
                active_model.update_time = Set(repo_info.update_time);
                active_model.commit = Set(repo_info.commit);
                Ok(relay_repo_info::Entity::update(active_model)
                    .exec(self.get_connection())
                    .await
                    .unwrap())
            }
            None => {
                relay_repo_info::Entity::insert(repo_info.clone().into_active_model())
                    .exec(self.get_connection())
                    .await
                    .unwrap();
                Ok(repo_info)
            }
        }
    }

    pub async fn insert_lfs_info(
        &self,
        lfs_info: relay_lfs_info::Model,
    ) -> Result<relay_lfs_info::Model, MegaError> {
        let list = self
            .get_lfs_info_by_origin_and_peerid(&lfs_info.origin, &lfs_info.peer_id)
            .await
            .unwrap();
        if list.is_empty() {
            relay_lfs_info::Entity::insert(lfs_info.clone().into_active_model())
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
    ) -> Result<Vec<relay_lfs_info::Model>, MegaError> {
        let model = relay_lfs_info::Entity::find()
            .filter(relay_lfs_info::Column::Origin.eq(origin))
            .filter(relay_lfs_info::Column::PeerId.eq(peer_id))
            .all(self.get_connection())
            .await;
        Ok(model?)
    }

    pub async fn get_all_lfs_info(&self) -> Result<Vec<relay_lfs_info::Model>, MegaError> {
        Ok(relay_lfs_info::Entity::find()
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn insert_nostr_event(
        &self,
        nostr_event: relay_nostr_event::Model,
    ) -> Result<InsertResult<relay_nostr_event::ActiveModel>, MegaError> {
        Ok(
            relay_nostr_event::Entity::insert(nostr_event.into_active_model())
                .exec(self.get_connection())
                .await
                .unwrap(),
        )
    }

    pub async fn get_nostr_event_by_id(
        &self,
        event_id: &str,
    ) -> Result<Option<relay_nostr_event::Model>, MegaError> {
        let result = relay_nostr_event::Entity::find_by_id(event_id)
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(result)
    }

    pub async fn get_all_nostr_event(&self) -> Result<Vec<relay_nostr_event::Model>, MegaError> {
        Ok(relay_nostr_event::Entity::find()
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_all_nostr_event_by_pubkey(
        &self,
        pubkey: &str,
    ) -> Result<Vec<relay_nostr_event::Model>, MegaError> {
        Ok(relay_nostr_event::Entity::find()
            .filter(relay_nostr_event::Column::Pubkey.eq(pubkey))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn insert_nostr_req(
        &self,
        nostr_req: relay_nostr_req::Model,
    ) -> Result<InsertResult<relay_nostr_req::ActiveModel>, MegaError> {
        Ok(
            relay_nostr_req::Entity::insert(nostr_req.into_active_model())
                .exec(self.get_connection())
                .await
                .unwrap(),
        )
    }

    pub async fn get_all_nostr_req_by_subscription_id(
        &self,
        subscription_id: &str,
    ) -> Result<Vec<relay_nostr_req::Model>, MegaError> {
        Ok(relay_nostr_req::Entity::find()
            .filter(relay_nostr_req::Column::SubscriptionId.eq(subscription_id))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_all_nostr_req(&self) -> Result<Vec<relay_nostr_req::Model>, MegaError> {
        Ok(relay_nostr_req::Entity::find()
            .all(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn save_alias_mapping(
        &self,
        model: relay_path_mapping::Model,
    ) -> Result<(), MegaError> {
        relay_path_mapping::Entity::insert(model.into_active_model())
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
    ) -> Result<Option<relay_path_mapping::Model>, MegaError> {
        Ok(relay_path_mapping::Entity::find()
            .filter(relay_path_mapping::Column::Alias.eq(alias))
            .one(self.get_connection())
            .await
            .unwrap())
    }
}
