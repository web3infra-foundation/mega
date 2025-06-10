pub mod git_db_storage;
pub mod init;
pub mod issue_storage;
pub mod lfs_db_storage;
pub mod mono_storage;
pub mod mq_storage;
pub mod mr_storage;
pub mod raw_db_storage;
pub mod relay_storage;
pub mod user_storage;

use sea_orm::{sea_query::OnConflict, ActiveModelTrait, ConnectionTrait, DbErr, EntityTrait};

use common::errors::MegaError;

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
pub async fn batch_save_model<E, A>(
    connection: &impl ConnectionTrait,
    save_models: Vec<A>,
) -> Result<(), MegaError>
where
    E: EntityTrait,
    A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
{
    batch_save_model_with_conflict(
        connection,
        save_models,
        OnConflict::new().do_nothing().to_owned(),
    )
    .await
}

/// Performs batch saving of models in the database with conflict resolution.
///
/// This function allows saving models in batches while specifying conflict resolution behavior using the `OnConflict` parameter.
/// It is intended for advanced use cases where fine-grained control over conflict handling is required.
///
/// # Arguments
///
/// * `connection` - A reference to the database connection.
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
pub async fn batch_save_model_with_conflict<E, A>(
    connection: &impl ConnectionTrait,
    save_models: Vec<A>,
    onconflict: OnConflict,
) -> Result<(), MegaError>
where
    E: EntityTrait,
    A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
{
    // notice that sqlx not support packets larger than 16MB now
    let futures = save_models.chunks(1000).map(|chunk| {
        let insert = E::insert_many(chunk.iter().cloned()).on_conflict(onconflict.clone());
        let conn = connection;
        async move {
            match insert.exec(conn).await {
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
