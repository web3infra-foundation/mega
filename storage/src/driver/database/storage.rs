//!
//!
//！

extern crate common;

use std::env;
use std::path::Path;
use std::path::PathBuf;

use async_trait::async_trait;
use entity::model::query_result::SelectResult;
use sea_orm::sea_query::OnConflict;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::ConnectionTrait;
use sea_orm::DatabaseConnection;
use sea_orm::DatabaseTransaction;
use sea_orm::DbErr;
use sea_orm::EntityTrait;
use sea_orm::IntoActiveModel;
use sea_orm::QueryFilter;
use sea_orm::QuerySelect;
use sea_orm::Set;
use sea_orm::TryIntoModel;

use common::errors::MegaError;
use entity::commit;
use entity::issue;
use entity::locks;
use entity::meta;
use entity::mr;
use entity::mr_info;
use entity::node;
use entity::objects;
use entity::pull_request;
use entity::refs;
use entity::repo_directory;

use crate::driver::file_storage;

#[async_trait]
pub trait ObjectStorage: Send + Sync {
    fn get_connection(&self) -> &DatabaseConnection;

    async fn save_mr_objects(
        &self,
        txn: Option<&DatabaseTransaction>,
        objects: Vec<mr::ActiveModel>,
    ) -> Result<bool, MegaError> {
        match txn {
            Some(txn) => batch_save_model(txn, objects).await?,
            None => batch_save_model(self.get_connection(), objects).await?,
        }
        Ok(true)
    }

    async fn save_obj_data(
        &self,
        txn: Option<&DatabaseTransaction>,
        mut obj_data: Vec<objects::ActiveModel>,
    ) -> Result<bool, MegaError> {
        let threshold = env::var("MEGA_BIG_OBJ_THRESHOLD_SIZE")
            .expect("MEGA_BIG_OBJ_THRESHOLD_SIZE not configured")
            .parse::<usize>()
            .unwrap();

        let fs_storage = file_storage::init("git-objects".to_owned()).await;

        let mut new_obj_data: Vec<objects::ActiveModel> = Vec::new();
        for model in obj_data.iter_mut() {
            let mut obj = model.clone().try_into_model().unwrap();
            if obj.data.len() / 1024 > threshold {
                let path = fs_storage
                    .put(&obj.git_id, obj.data.len() as i64, &obj.data)
                    .await
                    .unwrap();
                obj.link = Some(path);
                obj.data.clear();
            }
            new_obj_data.push(obj.into_active_model())
        }
        self.save_obj_data_to_db(txn, new_obj_data).await
    }

    async fn save_obj_data_to_db(
        &self,
        txn: Option<&DatabaseTransaction>,
        obj_data: Vec<objects::ActiveModel>,
    ) -> Result<bool, MegaError>;

    async fn get_mr_objects_by_type(
        &self,
        mr_id: i64,
        object_type: &str,
    ) -> Result<Vec<mr::Model>, MegaError> {
        Ok(mr::Entity::find()
            .filter(mr::Column::MrId.eq(mr_id))
            .filter(mr::Column::ObjectType.eq(object_type))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    async fn save_mr_info(&self, mr_info: mr_info::ActiveModel) -> Result<bool, MegaError> {
        mr_info::Entity::insert(mr_info)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(true)
    }

    async fn get_mr_infos(&self, mr_ids: Vec<i64>) -> Result<Vec<mr_info::Model>, MegaError> {
        Ok(mr_info::Entity::find()
            .filter(mr_info::Column::MrId.is_in(mr_ids))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    async fn get_obj_data_by_ids(
        &self,
        git_ids: Vec<String>,
    ) -> Result<Vec<objects::Model>, MegaError> {
        let mut objs: Vec<objects::Model> =
            batch_query_by_columns::<objects::Entity, objects::Column>(
                self.get_connection(),
                objects::Column::GitId,
                git_ids,
            )
            .await
            .unwrap();
        let fs_storage = file_storage::init("git-objects".to_owned()).await;

        for obj in objs.iter_mut() {
            if obj.link.is_some() {
                let data = fs_storage.get(&obj.git_id).await.unwrap();
                obj.data = data.to_vec();
            }
        }
        Ok(objs)
    }

    async fn get_obj_data_by_id(&self, git_id: &str) -> Result<Option<objects::Model>, MegaError> {
        let obj = objects::Entity::find()
            .filter(objects::Column::GitId.eq(git_id))
            .one(self.get_connection())
            .await
            .unwrap();

        if let Some(mut model) = obj {
            if model.link.is_some() {
                let fs_storage = file_storage::init("git-objects".to_owned()).await;
                let data = fs_storage.get(&model.git_id).await.unwrap();
                model.data = data.to_vec();
            }
            return Ok(Some(model));
        }
        Ok(None)
    }

    async fn get_all_refs_by_path(&self, repo_path: &str) -> Result<Vec<refs::Model>, MegaError> {
        // assuming HEAD points to branch master.
        Ok(refs::Entity::find()
            .filter(refs::Column::RepoPath.eq(repo_path))
            .all(self.get_connection())
            .await
            .unwrap())
    }

    async fn get_commit_by_hash(&self, hash: &str) -> Result<Option<commit::Model>, MegaError> {
        Ok(commit::Entity::find()
            .filter(commit::Column::GitId.eq(hash))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    async fn get_commit_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<commit::Model>, MegaError> {
        Ok(batch_query_by_columns::<commit::Entity, commit::Column>(
            self.get_connection(),
            commit::Column::GitId,
            hashes,
        )
        .await
        .unwrap())
    }

    async fn get_all_commits_by_path(
        &self,
        repo_path: &str,
    ) -> Result<Vec<commit::Model>, MegaError> {
        let commits: Vec<commit::Model> = commit::Entity::find()
            .filter(commit::Column::RepoPath.eq(repo_path))
            .all(self.get_connection())
            .await
            .unwrap();
        Ok(commits)
    }

    async fn search_refs(&self, path_str: &str) -> Result<Vec<refs::Model>, MegaError>;

    async fn search_commits(&self, path_str: &str) -> Result<Vec<commit::Model>, MegaError>;

    async fn save_refs(&self, save_models: Vec<refs::ActiveModel>) -> Result<bool, MegaError> {
        refs::Entity::insert_many(save_models)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(true)
    }

    async fn update_refs(&self, old_id: String, new_id: String, path: &Path) {
        let ref_data: Option<refs::Model> = refs::Entity::find()
            .filter(refs::Column::RefGitId.eq(old_id))
            .filter(refs::Column::RepoPath.eq(path.to_str().unwrap()))
            .one(self.get_connection())
            .await
            .unwrap();
        let mut ref_data: refs::ActiveModel = ref_data.unwrap().into();
        ref_data.ref_git_id = Set(new_id);
        ref_data.updated_at = Set(chrono::Utc::now().naive_utc());
        ref_data.update(self.get_connection()).await.unwrap();
    }

    async fn delete_refs(&self, old_id: String, path: &Path) {
        let delete_ref = refs::ActiveModel {
            ref_git_id: Set(old_id),
            repo_path: Set(path.to_str().unwrap().to_owned()),
            ..Default::default()
        };
        refs::Entity::delete(delete_ref)
            .exec(self.get_connection())
            .await
            .unwrap();
    }

    async fn get_nodes_by_hashes(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<node::Model>, MegaError> {
        Ok(batch_query_by_columns::<node::Entity, node::Column>(
            self.get_connection(),
            node::Column::GitId,
            hashes,
        )
        .await
        .unwrap())
    }

    async fn get_node_by_hash(&self, hash: &str) -> Result<Option<node::Model>, MegaError> {
        Ok(node::Entity::find()
            .filter(node::Column::GitId.eq(hash))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    async fn get_node_by_path(&self, path: &Path) -> Result<Vec<node::Model>, MegaError> {
        Ok(node::Entity::find()
            .filter(node::Column::RepoPath.eq(path.to_str().unwrap()))
            .all(self.get_connection())
            .await
            .unwrap())
    }
    async fn get_nodes(&self) -> Result<Vec<node::Model>, MegaError> {
        Ok(node::Entity::find()
            .select_only()
            .columns([
                node::Column::GitId,
                node::Column::Size,
                node::Column::FullPath,
            ])
            .all(self.get_connection())
            .await
            .unwrap())
    }

    async fn save_nodes(
        &self,
        txn: Option<&DatabaseTransaction>,
        nodes: Vec<node::ActiveModel>,
    ) -> Result<bool, MegaError> {
        match txn {
            Some(txn) => batch_save_model(txn, nodes).await.map(|_| true),
            None => batch_save_model(self.get_connection(), nodes)
                .await
                .map(|_| true),
        }
    }

    async fn save_commits(
        &self,
        txn: Option<&DatabaseTransaction>,
        commits: Vec<commit::ActiveModel>,
    ) -> Result<bool, MegaError> {
        match txn {
            Some(txn) => batch_save_model(txn, commits).await.map(|_| true),
            None => batch_save_model(self.get_connection(), commits)
                .await
                .map(|_| true),
        }
    }
    async fn search_root_node_by_path(&self, repo_path: &Path) -> Option<node::Model> {
        tracing::debug!("file_name: {:?}", repo_path.file_name());
        let res = node::Entity::find()
            .filter(node::Column::Name.eq(repo_path.file_name().unwrap().to_str().unwrap()))
            .one(self.get_connection())
            .await
            .unwrap();
        if let Some(res) = res {
            Some(res)
        } else {
            node::Entity::find()
                // .filter(node::Column::Path.eq(repo_path.to_str().unwrap()))
                .filter(node::Column::Name.eq(""))
                .one(self.get_connection())
                .await
                .unwrap()
        }
    }

    async fn get_meta_by_id(&self, oid: String) -> Result<Option<meta::Model>, MegaError> {
        let result = meta::Entity::find_by_id(oid)
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(result)
    }

    async fn delete_meta_by_id(&self, oid: String) -> Result<(), MegaError> {
        meta::Entity::delete_by_id(oid)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    async fn get_lock_by_id(&self, refspec: &str) -> Result<Option<locks::Model>, MegaError> {
        let result = locks::Entity::find_by_id(refspec)
            .one(self.get_connection())
            .await
            .unwrap();
        Ok(result)
    }

    async fn delete_lock_by_id(&self, id: String) {
        locks::Entity::delete_by_id(id)
            .exec(self.get_connection())
            .await
            .unwrap();
    }

    async fn save_issue(&self, issue: issue::ActiveModel) -> Result<bool, MegaError> {
        issue::Entity::insert(issue)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(true)
    }

    async fn update_issue(&self, issue: issue::ActiveModel) -> Result<bool, MegaError> {
        issue::Entity::update(issue)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(true)
    }

    async fn get_issue_by_id(&self, id: i64) -> Result<Option<issue::Model>, MegaError> {
        Ok(issue::Entity::find()
            .filter(issue::Column::Id.eq(id))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    async fn init_repo_dir(&self) -> Result<(), MegaError> {
        let pid = if let Some(root) = self.get_directory_by_full_path("/").await.unwrap() {
            root.id
        } else {
            let root = repo_directory::new(0, "root", "/");
            self.save_directory(root).await?
        };

        let init_dirs = env::var("MEGA_INIT_DIRS").unwrap();
        let mut model_vec = Vec::new();
        for str in init_dirs.split(',') {
            let path = PathBuf::from(str);
            model_vec.push(repo_directory::new(
                pid,
                path.file_name().unwrap().to_str().unwrap(),
                &format!("/{}", str),
            ));
        }
        repo_directory::Entity::insert_many(model_vec)
            .on_conflict(
                OnConflict::column(repo_directory::Column::FullPath)
                    .update_columns([
                        repo_directory::Column::Pid,
                        repo_directory::Column::Name,
                        repo_directory::Column::IsRepo,
                        repo_directory::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(())
    }

    async fn save_directory(&self, model: repo_directory::ActiveModel) -> Result<i32, MegaError> {
        Ok(repo_directory::Entity::insert(model)
            .exec(self.get_connection())
            .await
            .unwrap()
            .last_insert_id)
    }

    async fn get_directory_by_full_path(
        &self,
        path: &str,
    ) -> Result<Option<repo_directory::Model>, DbErr> {
        repo_directory::Entity::find()
            .filter(repo_directory::Column::FullPath.eq(path))
            .one(self.get_connection())
            .await
    }

    async fn get_directory_by_pid(&self, pid: i32) -> Result<Vec<repo_directory::Model>, DbErr> {
        repo_directory::Entity::find()
            .filter(repo_directory::Column::Pid.eq(pid))
            .all(self.get_connection())
            .await
    }
    async fn save_pull_request(
        &self,
        pull_request: pull_request::ActiveModel,
    ) -> Result<bool, MegaError> {
        pull_request::Entity::insert(pull_request)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(true)
    }

    async fn update_pull_request(
        &self,
        pull_request: pull_request::ActiveModel,
    ) -> Result<bool, MegaError> {
        pull_request::Entity::update(pull_request)
            .exec(self.get_connection())
            .await
            .unwrap();
        Ok(true)
    }

    async fn get_pull_request_by_id(
        &self,
        id: i64,
    ) -> Result<Option<pull_request::Model>, MegaError> {
        Ok(pull_request::Entity::find()
            .filter(pull_request::Column::Id.eq(id))
            .one(self.get_connection())
            .await
            .unwrap())
    }

    async fn count_obj_from_commit_and_node(
        &self,
        repo_path: &str,
    ) -> Result<Vec<SelectResult>, MegaError> {
        let select = node::Entity::find()
            .select_only()
            .filter(node::Column::RepoPath.eq(repo_path))
            .column(node::Column::NodeType)
            .column_as(node::Column::NodeType.count(), "count")
            .group_by(node::Column::NodeType);
        // .into_json();
        // .all(self.get_connection())
        // .await
        // .unwrap();
        let results = select
            .into_model::<SelectResult>()
            .all(self.get_connection())
            .await
            .unwrap();
        Ok(results)
    }
}

/// Performs batch saving of models in the database.
///
/// The method takes a vector of models to be saved and performs batch inserts using the given entity type `E`.
/// The models should implement the `ActiveModelTrait` trait, which provides the necessary functionality for saving and inserting the models.
///
/// The method splits the models into smaller chunks, each containing up to 100 models, and inserts them into the database using the `E::insert_many` function.
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

async fn batch_query_by_columns<T, C>(
    connection: &DatabaseConnection,
    column: C,
    ids: Vec<String>,
) -> Result<Vec<T::Model>, MegaError>
where
    T: EntityTrait,
    C: ColumnTrait,
{
    let mut result = Vec::<T::Model>::new();
    for chunk in ids.chunks(1000) {
        result.extend(
            T::find()
                .filter(column.is_in(chunk))
                .all(connection)
                .await
                .unwrap(),
        );
    }
    Ok(result)
}
