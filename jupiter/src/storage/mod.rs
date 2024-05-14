pub mod git_db_storage;
pub mod git_fs_storage;
pub mod init;
pub mod lfs_storage;
pub mod mega_storage;

use async_trait::async_trait;

use common::errors::MegaError;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter,
};
use venus::{
    import_repo::import_refs::{RefCommand, Refs},
    import_repo::repo::Repo,
};

///
/// This interface is designed to handle the commonalities between the database storage and
/// file system storage.
///
#[async_trait]
pub trait GitStorageProvider: Send + Sync {
    async fn save_ref(&self, repo: &Repo, refs: &RefCommand) -> Result<(), MegaError>;

    async fn remove_ref(&self, repo: &Repo, refs: &RefCommand) -> Result<(), MegaError>;

    async fn get_ref(&self, repo: &Repo) -> Result<Vec<Refs>, MegaError>;

    async fn update_ref(&self, repo: &Repo, ref_name: &str, new_id: &str) -> Result<(), MegaError>;

    // async fn save_entry(&self, repo: &Repo, entry_list: Vec<Entry>) -> Result<(), MegaError>;

    // async fn get_entry_by_sha1(
    //     &self,
    //     repo: Repo,
    //     sha1_vec: Vec<&str>,
    // ) -> Result<Vec<Entry>, MegaError>;
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

pub async fn batch_save_model_with_conflict<E, A>(
    connection: &impl ConnectionTrait,
    save_models: Vec<A>,
    onconflict: OnConflict,
) -> Result<(), MegaError>
where
    E: EntityTrait,
    A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
{
    let mut results = Vec::new();
    for chunk in save_models.chunks(1000) {
        // notice that sqlx not support packets larger than 16MB now
        let res = E::insert_many(chunk.iter().cloned())
            .on_conflict(onconflict.clone())
            .exec(connection);
        results.push(res);
    }
    futures::future::join_all(results).await;
    Ok(())
}

#[allow(unused)]
pub async fn batch_query_by_columns<T, C>(
    connection: &DatabaseConnection,
    column: C,
    ids: Vec<String>,
    filter_column: Option<C>,
    value: Option<String>,
) -> Result<Vec<T::Model>, MegaError>
where
    T: EntityTrait,
    C: ColumnTrait,
{
    let mut result = Vec::<T::Model>::new();
    for chunk in ids.chunks(1000) {
        let query_builder = T::find().filter(column.is_in(chunk));

        // Conditionally add the filter based on the value parameter
        let query_builder = match value {
            Some(ref v) => query_builder.filter(filter_column.unwrap().eq(v)),
            None => query_builder,
        };

        result.extend(query_builder.all(connection).await?);
    }
    Ok(result)
}
