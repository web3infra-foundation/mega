use async_trait::async_trait;
use common::errors::MegaError;
use entity::{commit, git_obj, refs};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::driver::{batch_save_model, ObjectStorage};

#[derive(Debug, Default)]
pub struct PgStorage {
    pub connection: DatabaseConnection,
}

impl PgStorage {
    pub fn new(connection: DatabaseConnection) -> PgStorage {
        PgStorage { connection }
    }
}

#[async_trait]
impl ObjectStorage for PgStorage {
    fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    async fn save_obj_data(&self, obj_data: Vec<git_obj::ActiveModel>) -> Result<bool, MegaError> {
        batch_save_model(self.get_connection(), obj_data).await?;
        Ok(true)
    }

    async fn search_refs(&self, path_str: &str) -> Result<Vec<refs::Model>, MegaError> {
        Ok(refs::Entity::find()
            .filter(refs::Column::RepoPath.contains(path_str))
            .all(&self.connection)
            .await?)
    }

    async fn search_commits(&self, path_str: &str) -> Result<Vec<commit::Model>, MegaError> {
        Ok(commit::Entity::find()
            .filter(commit::Column::RepoPath.contains(path_str))
            .all(&self.connection)
            .await?)
    }
}
