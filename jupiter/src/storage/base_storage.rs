use std::sync::Arc;

use async_trait::async_trait;
use common::errors::MegaError;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait,
    sea_query::OnConflict,
};
use sea_orm_migration::SchemaManagerConnection;

#[async_trait]
pub trait StorageConnector {
    const BATCH_CHUNK_SIZE: usize = 1000;

    fn get_connection(&self) -> &DatabaseConnection;

    fn mock() -> Self;

    fn new(connection: Arc<DatabaseConnection>) -> Self;

    fn build_connection_with_txn<'a>(
        &'a self,
        txn: Option<&'a DatabaseTransaction>,
    ) -> SchemaManagerConnection<'a> {
        if let Some(txn) = txn {
            SchemaManagerConnection::Transaction(txn)
        } else {
            SchemaManagerConnection::Connection(self.get_connection())
        }
    }

    /// Performs batch saving of models in the database.
    ///
    /// The method takes a vector of models to be saved and performs batch inserts using the given entity type `E`.
    /// The models should implement the `ActiveModelTrait` trait, which provides the necessary functionality for saving and inserting the models.
    ///
    /// The method splits the models into smaller chunks, each containing models configured by chunk_size, and inserts them into the database using the `E::insert_many` function.
    /// The results of each insertion are collected into a vector of futures.
    ///
    /// Note: Currently, SQLx does not support packets larger than 16MB.
    /// # Arguments
    ///
    /// * `save_models` - A vector of models to be saved.
    ///
    /// # Generic Constraints
    ///
    /// * `E` - The entity type that implements the `EntityTrait` trait.
    /// * `A` - The model type that implements the `ActiveModelTrait` trait and is convertible from the corresponding model type of `E`.
    ///
    /// # Errors
    ///
    /// Returns a `MegaError` if an error occurs during the batch save operation.
    async fn batch_save_model<E, A>(&self, save_models: Vec<A>) -> Result<(), MegaError>
    where
        E: EntityTrait,
        A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
    {
        let onconflict = OnConflict::new().do_nothing().to_owned();
        Self::batch_save_model_with_conflict(self, save_models, onconflict).await
    }

    async fn batch_save_model_with_txn<E, A>(
        &self,
        save_models: Vec<A>,
        txn: Option<&DatabaseTransaction>,
    ) -> Result<(), MegaError>
    where
        E: EntityTrait,
        A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
    {
        let onconflict = OnConflict::new().do_nothing().to_owned();
        Self::batch_save_model_with_conflict_and_txn(self, save_models, onconflict, txn).await
    }

    async fn batch_save_model_with_conflict_and_txn<E, A>(
        &self,
        save_models: Vec<A>,
        onconflict: OnConflict,
        txn: Option<&DatabaseTransaction>,
    ) -> Result<(), MegaError>
    where
        E: EntityTrait,
        A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
    {
        let conn = self.build_connection_with_txn(txn);

        let mut i = 0;
        let len = save_models.len();

        while i < len {
            let end = (i + Self::BATCH_CHUNK_SIZE).min(len);
            let models = save_models[i..end].to_vec();
            let _ = match E::insert_many(models)
                .on_conflict(onconflict.clone())
                .exec(&conn)
                .await
            {
                Ok(_) => Ok(()),
                Err(DbErr::RecordNotInserted) => Ok(()),
                Err(e) => Err(e),
            };
            i = end;
        }
        Ok(())
    }

    /// Performs batch saving of models in the database with conflict resolution.
    ///
    /// This function allows saving models in batches while specifying conflict resolution behavior using the `OnConflict` parameter.
    /// It is intended for advanced use cases where fine-grained control over conflict handling is required.
    ///
    /// # Arguments
    ///
    /// * `save_models` - A vector of models to be saved.
    /// * `onconflict` - Specifies the conflict resolution strategy to be used during insertion.
    ///
    /// # Generic Constraints
    ///
    /// * `E` - The entity type that implements the `EntityTrait` trait.
    /// * `A` - The model type that implements the `ActiveModelTrait` trait and is convertible from the corresponding model type of `E`.
    ///
    /// # Errors
    ///
    /// Returns a `MegaError` if an error occurs during the batch save operation.
    /// Note: The function ignores `DbErr::RecordNotInserted` errors, which may lead to silent failures.
    /// Use this function with caution and ensure that the `OnConflict` parameter is configured correctly to avoid unintended consequences.
    async fn batch_save_model_with_conflict<E, A>(
        &self,
        save_models: Vec<A>,
        onconflict: OnConflict,
    ) -> Result<(), MegaError>
    where
        E: EntityTrait,
        A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
    {
        let futures = save_models.chunks(Self::BATCH_CHUNK_SIZE).map(|chunk| {
            let insert = E::insert_many(chunk.iter().cloned()).on_conflict(onconflict.clone());

            async move {
                match insert.exec(self.get_connection()).await {
                    Ok(_) => Ok(()),
                    Err(DbErr::RecordNotInserted) => {
                        // ignore not inserted err
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
        });
        futures::future::try_join_all(futures).await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BaseStorage {
    pub connection: Arc<DatabaseConnection>,
}

impl StorageConnector for BaseStorage {
    fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    fn mock() -> Self {
        Self {
            connection: Arc::new(DatabaseConnection::default()),
        }
    }

    fn new(connection: Arc<DatabaseConnection>) -> Self {
        Self { connection }
    }
}
