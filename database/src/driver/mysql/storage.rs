//!
//!
//!
//!

use async_trait::async_trait;

use entity::commit;

use entity::obj_data;
use entity::refs;

use sea_orm::DatabaseBackend;
use sea_orm::DatabaseConnection;
use sea_orm::EntityTrait;

use sea_orm::Statement;
use sea_orm::TryIntoModel;

use crate::driver::batch_save_model;

use crate::driver::MegaError;
use crate::driver::ObjectStorage;

#[derive(Debug, Default)]
pub struct MysqlStorage {
    pub connection: DatabaseConnection,
}

impl MysqlStorage {
    pub fn new(connection: DatabaseConnection) -> MysqlStorage {
        MysqlStorage { connection }
    }
}

#[async_trait]
impl ObjectStorage for MysqlStorage {
    fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    async fn save_obj_data(&self, obj_data: Vec<obj_data::ActiveModel>) -> Result<bool, MegaError> {
        let packet_size = obj_data
            .iter()
            .map(|model| model.clone().try_into_model().unwrap().data.len())
            .sum::<usize>();

        if packet_size > 0xDF_FF_FF {
            let mut batch_obj = Vec::new();
            let mut sum = 0;
            for model in obj_data {
                let size = model.data.as_ref().len();
                if sum + size < 0xDF_FF_FF {
                    sum += size;
                    batch_obj.push(model);
                } else {
                    batch_save_model(self.get_connection(), batch_obj).await?;
                    sum = size;
                    batch_obj = vec![model];
                }
            }
            if !batch_obj.is_empty() {
                batch_save_model(self.get_connection(), batch_obj).await?;
            }
        } else {
            batch_save_model(self.get_connection(), obj_data).await?;
        }
        Ok(true)
    }

    async fn search_refs(&self, path_str: &str) -> Result<Vec<refs::Model>, MegaError> {
        Ok(refs::Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DatabaseBackend::MySql,
                r#"SELECT * FROM refs where ? LIKE CONCAT(repo_path, '%') "#,
                [path_str.into()],
            ))
            .all(&self.connection)
            .await?)
    }

    async fn search_commits(&self, path_str: &str) -> Result<Vec<commit::Model>, MegaError> {
        Ok(commit::Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DatabaseBackend::MySql,
                r#"SELECT * FROM commit where ? LIKE CONCAT(repo_path, '%')"#,
                [path_str.into()],
            ))
            .all(&self.connection)
            .await?)
    }
}
