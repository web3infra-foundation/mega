use std::{env, sync::Arc};

use async_trait::async_trait;
use sea_orm::PaginatorTrait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};

use callisto::import_refs;
use common::errors::MegaError;
use venus::internal::object::GitObjectModel;
use venus::internal::pack::entry::Entry;
use venus::internal::pack::reference::RefCommand;
use venus::internal::pack::reference::Refs;
use venus::repo::Repo;

use crate::{
    raw_storage::{self, RawStorage},
    storage::GitStorageProvider,
};

use super::batch_save_model;

#[derive(Clone)]
pub struct GitDbStorage {
    pub raw_storage: Arc<dyn RawStorage>,
    pub connection: Arc<DatabaseConnection>,
    pub raw_obj_threshold: usize,
}

#[async_trait]
impl GitStorageProvider for GitDbStorage {
    async fn save_ref(&self, repo: &Repo, refs: &RefCommand) -> Result<(), MegaError> {
        let mut model: import_refs::Model = refs.clone().into();
        model.repo_id = repo.repo_id;
        let a_model = model.into_active_model();
        import_refs::Entity::insert(a_model)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    async fn remove_ref(&self, repo: &Repo, refs: &RefCommand) -> Result<(), MegaError> {
        import_refs::Entity::delete_many()
            .filter(import_refs::Column::RepoId.eq(repo.repo_id))
            .filter(import_refs::Column::RefName.eq(refs.ref_name.clone()))
            .exec(self.get_connection())
            .await?;
        Ok(())
    }

    async fn get_ref(&self, repo: &Repo) -> Result<Vec<Refs>, MegaError> {
        let result = import_refs::Entity::find()
            .filter(import_refs::Column::RepoId.eq(repo.repo_id))
            .all(self.get_connection())
            .await?;
        let res: Vec<Refs> = result.into_iter().map(|x| x.into()).collect();
        Ok(res)
    }

    async fn update_ref(&self, repo: &Repo, ref_name: &str, new_id: &str) -> Result<(), MegaError> {
        let ref_data: import_refs::Model = import_refs::Entity::find()
            .filter(import_refs::Column::RepoId.eq(repo.repo_id))
            .filter(import_refs::Column::RefName.eq(ref_name))
            .one(self.get_connection())
            .await
            .unwrap()
            .unwrap();
        let mut ref_data: import_refs::ActiveModel = ref_data.into();
        ref_data.ref_git_id = Set(new_id.to_string());
        ref_data.updated_at = Set(chrono::Utc::now().naive_utc());
        ref_data.update(self.get_connection()).await.unwrap();
        Ok(())
    }
}

impl GitDbStorage {
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>) -> Self {
        let raw_obj_threshold = env::var("MEGA_BIG_OBJ_THRESHOLD_SIZE")
            .expect("MEGA_BIG_OBJ_THRESHOLD_SIZE not configured")
            .parse::<usize>()
            .unwrap();
        let storage_type = env::var("MEGA_RAW_STORAGE").unwrap();
        let path = env::var("MEGA_OBJ_LOCAL_PATH").unwrap();
        GitDbStorage {
            connection,
            raw_storage: raw_storage::init(storage_type, path).await,
            raw_obj_threshold,
        }
    }

    pub fn mock() -> Self {
        GitDbStorage {
            connection: Arc::new(DatabaseConnection::default()),
            raw_storage: raw_storage::mock(),
            raw_obj_threshold: 1024,
        }
    }

    pub async fn default_branch_exist(&self, repo: &Repo) -> Result<bool, MegaError> {
        let result = import_refs::Entity::find()
            .filter(import_refs::Column::RepoId.eq(repo.repo_id))
            .filter(import_refs::Column::DefaultBranch.eq(true))
            .count(self.get_connection())
            .await?;
        Ok(result > 0)
    }

    pub async fn save_entry(
        &self,
        repo: &Repo,
        entry_list: Vec<Entry>,
    ) -> Result<(), MegaError> {
        let mut commits = Vec::new();
        let mut trees = Vec::new();
        let mut blobs = Vec::new();
        let mut raw_blobs = Vec::new();
        let mut tags = Vec::new();

        for entry in entry_list {
            let raw_obj = entry.process_entry();
            let model = raw_obj.convert_to_mega_model();
            match model {
                GitObjectModel::Commit(mut commit) => {
                    commit.repo_id = repo.repo_id;
                    commits.push(commit.into_active_model())
                },
                GitObjectModel::Tree(mut tree) => {
                    tree.repo_id = repo.repo_id;
                    trees.push(tree.clone().into_active_model());
                }
                GitObjectModel::Blob(mut blob, raw) => {
                    blob.repo_id = repo.repo_id;
                    blobs.push(blob.clone().into_active_model());
                    raw_blobs.push(raw.into_active_model());
                }
                GitObjectModel::Tag(mut tag) => {
                    tag.repo_id = repo.repo_id;
                    tags.push(tag.into_active_model())
                },
            }
        }

        batch_save_model(self.get_connection(), commits)
            .await
            .unwrap();
        batch_save_model(self.get_connection(), trees)
            .await
            .unwrap();
        batch_save_model(self.get_connection(), blobs)
            .await
            .unwrap();
        batch_save_model(self.get_connection(), raw_blobs)
            .await
            .unwrap();
        batch_save_model(self.get_connection(), tags).await.unwrap();
        Ok(())
    }
}
