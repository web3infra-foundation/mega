//!
//!
//!
//!

use async_trait::async_trait;
use sea_orm::DatabaseBackend;
use sea_orm::DatabaseConnection;
use sea_orm::DatabaseTransaction;
use sea_orm::EntityTrait;
use sea_orm::Statement;
use sea_orm::TryIntoModel;

use common::errors::MegaError;
use entity::commit;
use entity::objects;
use entity::refs;

use crate::driver::database::storage::batch_save_model;
use crate::driver::database::storage::ObjectStorage;

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

    /// Asynchronously saves object data to the database, splitting it into multiple batches if necessary.
    ///
    /// This function takes an optional transaction `txn`, a database connection `conn`, and a vector of `git_obj::ActiveModel` objects.
    /// If the total size of objects exceeds a certain limit, the data is split into multiple batches for saving.
    ///
    /// # Arguments
    ///
    /// - `txn`: An optional database transaction.
    /// - `conn`: A database connection.
    /// - `obj_data`: A vector of `git_obj::ActiveModel` objects to be saved.
    ///
    /// # Returns
    ///
    /// - `Result<bool, MegaError>`: `Ok(true)` if the save operation is successful; otherwise, an error is returned.
    async fn save_obj_data_to_db(
        &self,
        txn: Option<&DatabaseTransaction>,
        obj_data: Vec<objects::ActiveModel>,
    ) -> Result<bool, MegaError> {
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
                    conditional_batch_save_model(txn, self.get_connection(), batch_obj).await?;
                    sum = size;
                    batch_obj = vec![model];
                }
            }
            if !batch_obj.is_empty() {
                conditional_batch_save_model(txn, self.get_connection(), batch_obj).await?;
            }
        } else {
            conditional_batch_save_model(txn, self.get_connection(), obj_data).await?;
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

/// Conditionally saves a batch of object data to the database.
///
/// This function is used when an optional transaction `txn` and a database connection `conn` are provided.
///
/// # Arguments
///
/// - `txn`: An optional database transaction.
/// - `conn`: A database connection.
/// - `obj_data`: A vector of `git_obj::ActiveModel` objects to be saved as a batch.
///
/// # Returns
///
/// - `Result<(), MegaError>`: `Ok(())` if the save operation is successful; otherwise, an error is returned.
async fn conditional_batch_save_model(
    txn: Option<&DatabaseTransaction>,
    conn: &DatabaseConnection,
    obj_data: Vec<objects::ActiveModel>,
) -> Result<(), MegaError> {
    match txn {
        Some(txn) => batch_save_model(txn, obj_data.to_vec()).await,
        None => batch_save_model(conn, obj_data.to_vec()).await,
    }
}
