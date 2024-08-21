use std::sync::Arc;

use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, InsertResult, IntoActiveModel, QueryFilter,
};

use callisto::{lfs_locks, lfs_objects, lfs_split_relations};
use common::errors::MegaError;

#[derive(Clone)]
pub struct LfsStorage {
    pub connection: Arc<DatabaseConnection>,
}

impl LfsStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>) -> Self {
        LfsStorage { connection }
    }

    pub fn mock() -> Self {
        LfsStorage {
            connection: Arc::new(DatabaseConnection::default()),
        }
    }

    pub async fn new_lfs_object(
        &self,
        object: lfs_objects::Model,
    ) -> Result<InsertResult<lfs_objects::ActiveModel>, MegaError> {
        Ok(lfs_objects::Entity::insert(object.into_active_model())
            .exec(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn new_lfs_relation(
        &self,
        relation: lfs_split_relations::Model,
    ) -> Result<InsertResult<lfs_split_relations::ActiveModel>, MegaError> {
        lfs_split_relations::Entity::insert(relation.into_active_model())
            .exec(self.get_connection())
            .await
            .map_err(|e| MegaError::with_message(e.to_string().as_str()))
    }

    pub async fn get_lfs_object(
        &self,
        oid: String,
    ) -> Result<Option<lfs_objects::Model>, MegaError> {
        let result = lfs_objects::Entity::find_by_id(oid)
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(result)
    }

    pub async fn get_lfs_relations(
        &self,
        oid: String,
    ) -> Result<Vec<lfs_split_relations::Model>, MegaError> {
        let obj = self.get_lfs_object(oid.clone()).await?;
        if obj.is_none() {
            return Err(MegaError::with_message("Object not found"));
        }
        let result = lfs_split_relations::Entity::find()
            .filter(lfs_split_relations::Column::OriOid.eq(oid))
            .all(self.get_connection())
            .await
            .unwrap();
        Ok(result)
    }

    pub async fn get_lfs_relations_ori_oid(
        &self,
        sub_oid: &String,
    ) -> Result<Vec<String>, MegaError> {
        let result = lfs_split_relations::Entity::find()
            .filter(lfs_split_relations::Column::SubOid.eq(sub_oid))
            .all(self.get_connection())
            .await
            .unwrap();
        Ok(result.iter().map(|r| r.ori_oid.clone()).collect())
    }

    pub async fn delete_lfs_object(&self, oid: String) -> Result<(), MegaError> {
        lfs_objects::Entity::delete_by_id(oid)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn delete_lfs_relation(
        &self,
        object: lfs_split_relations::Model,
    ) -> Result<(), MegaError> {
        let r: lfs_split_relations::ActiveModel = object.into();
        lfs_split_relations::Entity::delete(r)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn new_lock(
        &self,
        lfs_lock: lfs_locks::Model,
    ) -> Result<InsertResult<lfs_locks::ActiveModel>, MegaError> {
        Ok(lfs_locks::Entity::insert(lfs_lock.into_active_model())
            .exec(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn get_lock_by_id(
        &self,
        refspec: &str,
    ) -> Result<Option<lfs_locks::Model>, MegaError> {
        let result = lfs_locks::Entity::find_by_id(refspec)
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(result)
    }

    pub async fn update_lock(
        &self,
        lfs_lock: lfs_locks::Model,
    ) -> Result<lfs_locks::Model, MegaError> {
        Ok(lfs_locks::Entity::update(lfs_lock.into_active_model())
            .exec(self.get_connection())
            .await
            .unwrap())
    }

    pub async fn delete_lock_by_id(&self, id: String) {
        lfs_locks::Entity::delete_by_id(id)
            .exec(self.get_connection())
            .await
            .unwrap();
    }
}
