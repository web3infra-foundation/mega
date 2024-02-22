use std::{env, sync::Arc};

use async_trait::async_trait;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, IntoActiveModel, QueryFilter, Set,
};

use common::errors::MegaError;
use db_entity::{db_enums::StorageType, git_commit, git_refs, mega_commit, raw_objects};
use venus::internal::{
    object::commit::Commit,
    pack::{entry::Entry, reference::RefCommand},
    repo::Repo,
};

use crate::{
    raw_storage::{self, RawStorage},
    storage::StorageProvider,
};

use crate::storage::MegaStorageProvider;

pub struct MegaStorage {
    pub raw_storage: Arc<dyn RawStorage>,
    pub connection: DatabaseConnection,
}

#[async_trait]
impl StorageProvider for MegaStorage {
    async fn save_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError> {
        let mut model: git_refs::Model = refs.clone().into();
        model.ref_git_id = refs.new_id;
        model.repo_id = repo.repo_id;
        let a_model = model.into_active_model();
        git_refs::Entity::insert(a_model)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    async fn remove_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError> {
        git_refs::Entity::delete_many()
            .filter(git_refs::Column::RepoId.eq(repo.repo_id))
            .filter(git_refs::Column::RefName.eq(refs.ref_name))
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    async fn get_ref(&self, repo: Repo, refs: RefCommand) -> Result<String, MegaError> {
        let result = git_refs::Entity::find()
            .filter(git_refs::Column::RepoId.eq(repo.repo_id))
            .filter(git_refs::Column::RefName.eq(refs.ref_name))
            .one(self.get_connection())
            .await?;
        if let Some(model) = result {
            return Ok(model.ref_git_id);
        }
        Ok(String::new())
    }

    async fn update_ref(&self, repo: Repo, refs: RefCommand) -> Result<(), MegaError> {
        let ref_data: Option<git_refs::Model> = git_refs::Entity::find()
            .filter(git_refs::Column::RepoId.eq(repo.repo_id))
            .filter(git_refs::Column::RefName.eq(refs.ref_name))
            .one(self.get_connection())
            .await
            .unwrap();
        let mut ref_data: git_refs::ActiveModel = ref_data.unwrap().into();
        ref_data.ref_git_id = Set(refs.new_id);
        ref_data.updated_at = Set(chrono::Utc::now().naive_utc());
        ref_data.update(self.get_connection()).await.unwrap();
        Ok(())
    }

    async fn save_entry(&self, repo: Repo, result_entity: Vec<Entry>) -> Result<(), MegaError> {
        let threshold = env::var("MEGA_BIG_OBJ_THRESHOLD_SIZE")
            .expect("MEGA_BIG_OBJ_THRESHOLD_SIZE not configured")
            .parse::<usize>()
            .unwrap();

        let mut save_models: Vec<raw_objects::ActiveModel> = Vec::new();
        for entry in result_entity.iter() {
            let mut model: raw_objects::Model = entry.clone().into();
            let data = model.data.clone().unwrap();
            // save data through raw_storgae insted of database if exceed threshold
            if threshold != 0 && data.len() / 1024 > threshold {
                let b_link = self.raw_storage.put_entry(&repo.repo_name, entry).await.unwrap();
                model.storage_type = self.raw_storage.get_storage_type();
                model.data = Some(b_link);
            }
            save_models.push(model.into_active_model())
        }
        batch_save_model(self.get_connection(), save_models)
            .await
            .unwrap();
        Ok(())
    }

    async fn get_entry_by_sha1(&self, repo: Repo, sha1_vec: Vec<&str>) -> Result<Vec<Entry>, MegaError> {
        let models = raw_objects::Entity::find()
            .filter(raw_objects::Column::Sha1.is_in(sha1_vec))
            .all(self.get_connection())
            .await
            .unwrap();
        let mut result: Vec<Entry> = Vec::new();
        for mut model in models {
            if model.storage_type == StorageType::Database {
                result.push(model.into());
            } else {
                let data = self.raw_storage.get_object(&repo.repo_name, &model.sha1).await.unwrap();
                model.data = Some(data.to_vec());
                result.push(model.into());
            }
        }
        Ok(result)
    }
}

#[async_trait]
impl MegaStorageProvider for MegaStorage {
    async fn save_git_commits(
        &self,
        repo_id: i64,
        full_path: &str,
        commits: Vec<Commit>,
    ) -> Result<(), MegaError> {
        let git_commits: Vec<git_commit::Model> =
            commits.into_iter().map(git_commit::Model::from).collect();
        let mut save_models = Vec::new();
        for mut git_commit in git_commits {
            git_commit.full_path = full_path.to_string();
            git_commit.repo_id = repo_id;
            save_models.push(git_commit.into_active_model());
        }
        batch_save_model(self.get_connection(), save_models)
            .await
            .unwrap();
        Ok(())
    }

    async fn save_mega_commits(
        &self,
        mr_id: &str,
        full_path: &str,
        commits: Vec<Commit>,
    ) -> Result<(), MegaError> {
        let mega_commits: Vec<mega_commit::Model> =
            commits.into_iter().map(mega_commit::Model::from).collect();
        let mut save_models = Vec::new();
        for mut mega_commit in mega_commits {
            mega_commit.full_path = full_path.to_string();
            mega_commit.mr_id = Some(mr_id.to_string());
            save_models.push(mega_commit.into_active_model());
        }
        batch_save_model(self.get_connection(), save_models)
            .await
            .unwrap();
        Ok(())
    }
}

impl MegaStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: DatabaseConnection) -> Self {
        MegaStorage {
            connection,
            raw_storage: raw_storage::init().await,
        }
    }
}

/// Performs batch saving of models in the database.
///
/// The method takes a vector of models to be saved and performs batch inserts using the given entity type `E`.
/// The models should implement the `ActiveModelTrait` trait, which provides the necessary functionality for saving and inserting the models.
///
/// The method splits the models into smaller chunks, each containing models configured bu chunk_size, and inserts them into the database using the `E::insert_many` function.
/// The results of each insertion are collected into a vector of futures.
///
/// Note: Currently, SQLx does not support packets larger than 16MB.
///
///
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
    let mut results = Vec::new();
    for chunk in save_models.chunks(1000) {
        // notice that sqlx not support packets larger than 16MB now
        let res = E::insert_many(chunk.iter().cloned())
            .on_conflict(OnConflict::new().do_nothing().to_owned())
            .exec(connection);
        results.push(res);
    }
    futures::future::join_all(results).await;
    Ok(())
}

#[allow(unused)]
async fn batch_query_by_columns<T, C>(
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
