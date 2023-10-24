use async_trait::async_trait;
use common::errors::MegaError;
use entity::{commit, git_obj, refs};
use sea_orm::{
    ColumnTrait, DatabaseBackend, DatabaseConnection, DatabaseTransaction,
    EntityTrait, QueryFilter, Statement,
};

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

    async fn save_obj_data_to_db(
        &self,
        txn: Option<&DatabaseTransaction>,
        obj_data: Vec<git_obj::ActiveModel>,
    ) -> Result<bool, MegaError> {
        match txn {
            Some(txn) => batch_save_model(txn, obj_data).await?,
            None => batch_save_model(self.get_connection(), obj_data).await?,
        }
        Ok(true)
    }

    async fn search_refs(&self, path_str: &str) -> Result<Vec<refs::Model>, MegaError> {
        Ok(refs::Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"SELECT * FROM refs where $1 LIKE CONCAT(repo_path, '%') "#,
                [path_str.into()],
            ))
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
