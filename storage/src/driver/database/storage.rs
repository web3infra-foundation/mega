//!
//!
//！

extern crate common;

use std::cmp::min;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;

use entity::commit;
use entity::git_obj;
use entity::issue;
use entity::locks;
use entity::meta;
use entity::mr;
use entity::mr_info;
use entity::node;
use entity::pull_request;
use entity::refs;

use entity::repo_directory;
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

use crate::driver::fs::local_storage::MetaObject;
use crate::driver::fs::lfs_structs::Lock;
use crate::driver::fs::lfs_structs::RequestVars;
use common::errors::GitLFSError;
use common::errors::MegaError;

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
        mut obj_data: Vec<git_obj::ActiveModel>,
    ) -> Result<bool, MegaError> {
        let threshold = env::var("MEGA_BIG_OBJ_THRESHOLD_SIZE")
            .expect("MEGA_BIG_OBJ_THRESHOLD_SIZE not configured")
            .parse::<usize>()
            .unwrap();

        let storage_path = env::var("MEGA_BIG_OBJ_STORAGR_PATH")
            .expect("MEGA_BIG_OBJ_STORAGR_PATH not configured")
            .parse::<PathBuf>()
            .unwrap();
        let mut new_obj_data: Vec<git_obj::ActiveModel> = Vec::new();
        for model in obj_data.iter_mut() {
            let mut obj = model.clone().try_into_model().unwrap();
            if obj.data.len() / 1024 > threshold {
                let git_id = &obj.git_id;
                let mut full_path = storage_path.clone();
                full_path.push(&git_id[0..2]);
                full_path.push(&git_id[2..4]);
                fs::create_dir_all(&full_path).unwrap();
                full_path.push(git_id);
                let mut obj_file = File::create(&full_path).unwrap();
                obj_file.write_all(&obj.data).unwrap();
                obj.link = full_path.to_str().map(|s| s.to_string());
                obj.data.clear();
            }
            new_obj_data.push(obj.into_active_model())
        }
        self.save_obj_data_to_db(txn, new_obj_data).await
    }

    async fn save_obj_data_to_db(
        &self,
        txn: Option<&DatabaseTransaction>,
        obj_data: Vec<git_obj::ActiveModel>,
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

    fn get_obj_data_from_disk(&self, obj: &mut git_obj::Model) {
        if let Some(link) = &obj.link {
            let data = fs::read(link).unwrap();
            obj.data = data;
        }
    }

    async fn get_obj_data_by_ids(
        &self,
        git_ids: Vec<String>,
    ) -> Result<Vec<git_obj::Model>, MegaError> {
        let mut objs: Vec<git_obj::Model> =
            batch_query_by_columns::<git_obj::Entity, git_obj::Column>(
                self.get_connection(),
                git_obj::Column::GitId,
                git_ids,
            )
            .await
            .unwrap();
        for obj in objs.iter_mut() {
            if obj.data.is_empty() {
                self.get_obj_data_from_disk(obj);
            }
        }
        Ok(objs)
    }

    async fn get_obj_data_by_id(&self, git_id: &str) -> Result<Option<git_obj::Model>, MegaError> {
        let obj = git_obj::Entity::find()
            .filter(git_obj::Column::GitId.eq(git_id))
            .one(self.get_connection())
            .await
            .unwrap();
        if let Some(mut model) = obj {
            if model.data.is_empty() {
                self.get_obj_data_from_disk(&mut model);
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

    async fn save_nodes(&self, nodes: Vec<node::ActiveModel>) -> Result<bool, MegaError> {
        batch_save_model(self.get_connection(), nodes).await?;
        Ok(true)
    }

    async fn save_commits(&self, commits: Vec<commit::ActiveModel>) -> Result<bool, MegaError> {
        batch_save_model(self.get_connection(), commits)
            .await
            .unwrap();
        Ok(true)
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

    async fn lfs_get_meta(&self, v: &RequestVars) -> Result<MetaObject, GitLFSError> {
        let result = meta::Entity::find_by_id(v.oid.clone())
            .one(self.get_connection())
            .await
            .unwrap();

        match result {
            Some(val) => Ok(MetaObject {
                oid: val.oid,
                size: val.size,
                exist: val.exist,
            }),
            None => Err(GitLFSError::GeneralError("".to_string())),
        }
    }

    async fn lfs_put_meta(&self, v: &RequestVars) -> Result<MetaObject, GitLFSError> {
        // Check if already exist.
        let result = meta::Entity::find_by_id(v.oid.clone())
            .one(self.get_connection())
            .await
            .unwrap();
        if let Some(result) = result {
            return Ok(MetaObject {
                oid: result.oid,
                size: result.size,
                exist: true,
            });
        }

        // Put into database if not exist.
        let meta = MetaObject {
            oid: v.oid.to_string(),
            size: v.size,
            exist: true,
        };

        let meta_to = meta::ActiveModel {
            oid: Set(meta.oid.to_owned()),
            size: Set(meta.size.to_owned()),
            exist: Set(true),
        };

        let res = meta::Entity::insert(meta_to)
            .exec(self.get_connection())
            .await;
        match res {
            Ok(_) => Ok(meta),
            Err(err) => Err(GitLFSError::GeneralError(err.to_string())),
        }
    }

    async fn lfs_delete_meta(&self, v: &RequestVars) -> Result<(), GitLFSError> {
        let res = meta::Entity::delete_by_id(v.oid.to_owned())
            .exec(self.get_connection())
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(_) => Err(GitLFSError::GeneralError("".to_string())),
        }
    }

    async fn lfs_get_locks(&self, refspec: &str) -> Result<Vec<Lock>, GitLFSError> {
        let result = locks::Entity::find_by_id(refspec)
            .one(self.get_connection())
            .await
            .unwrap();

        match result {
            Some(val) => {
                let data = val.data;
                let locks: Vec<Lock> = serde_json::from_str(&data).unwrap();
                Ok(locks)
            }
            None => Err(GitLFSError::GeneralError("".to_string())),
        }
    }

    async fn lfs_get_filtered_locks(
        &self,
        refspec: &str,
        path: &str,
        cursor: &str,
        limit: &str,
    ) -> Result<(Vec<Lock>, String), GitLFSError> {
        let mut locks = match self.lfs_get_locks(refspec).await {
            Ok(locks) => locks,
            Err(_) => vec![],
        };

        println!("Locks retrieved: {:?}", locks);

        if !cursor.is_empty() {
            let mut last_seen = -1;
            for (i, v) in locks.iter().enumerate() {
                if v.id == *cursor {
                    last_seen = i as i32;
                    break;
                }
            }

            if last_seen > -1 {
                locks = locks.split_off(last_seen as usize);
            } else {
                // Cursor not found.
                return Err(GitLFSError::GeneralError("".to_string()));
            }
        }

        if !path.is_empty() {
            let mut filterd = Vec::<Lock>::new();
            for lock in locks.iter() {
                if lock.path == *path {
                    filterd.push(Lock {
                        id: lock.id.to_owned(),
                        path: lock.path.to_owned(),
                        owner: lock.owner.clone(),
                        locked_at: lock.locked_at.to_owned(),
                    });
                }
            }
            locks = filterd;
        }

        let mut next = "".to_string();
        if !limit.is_empty() {
            let mut size = limit.parse::<i64>().unwrap();
            size = min(size, locks.len() as i64);

            if size + 1 < locks.len() as i64 {
                next = locks[size as usize].id.to_owned();
            }
            let _ = locks.split_off(size as usize);
        }

        Ok((locks, next))
    }

    async fn lfs_add_lock(&self, repo: &str, locks: Vec<Lock>) -> Result<(), GitLFSError> {
        let result = locks::Entity::find_by_id(repo.to_owned())
            .one(self.get_connection())
            .await
            .unwrap();

        match result {
            // Update
            Some(val) => {
                let d = val.data.to_owned();
                let mut locks_from_data = if !d.is_empty() {
                    let locks_from_data: Vec<Lock> = serde_json::from_str(&d).unwrap();
                    locks_from_data
                } else {
                    vec![]
                };
                let mut locks = locks;
                locks_from_data.append(&mut locks);

                locks_from_data.sort_by(|a, b| {
                    a.locked_at
                        .partial_cmp(&b.locked_at)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let d = serde_json::to_string(&locks_from_data).unwrap();

                let mut lock_to: locks::ActiveModel = val.into();
                lock_to.data = Set(d.to_owned());
                let res = lock_to.update(self.get_connection()).await;
                match res.is_ok() {
                    true => Ok(()),
                    false => Err(GitLFSError::GeneralError("".to_string())),
                }
            }
            // Insert
            None => {
                let mut locks = locks;
                locks.sort_by(|a, b| {
                    a.locked_at
                        .partial_cmp(&b.locked_at)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let data = serde_json::to_string(&locks).unwrap();
                let lock_to = locks::ActiveModel {
                    id: Set(repo.to_owned()),
                    data: Set(data.to_owned()),
                };
                let res = locks::Entity::insert(lock_to)
                    .exec(self.get_connection())
                    .await;
                match res.is_ok() {
                    true => Ok(()),
                    false => Err(GitLFSError::GeneralError("".to_string())),
                }
            }
        }
    }

    async fn lfs_delete_lock(
        &self,
        repo: &str,
        _user: Option<String>,
        id: &str,
        force: bool,
    ) -> Result<Lock, GitLFSError> {
        let result = locks::Entity::find_by_id(repo.to_owned())
            .one(self.get_connection())
            .await
            .unwrap();

        match result {
            // Exist, then delete.
            Some(val) => {
                let d = val.data.to_owned();
                let locks_from_data = if !d.is_empty() {
                    let locks_from_data: Vec<Lock> = serde_json::from_str(&d).unwrap();
                    locks_from_data
                } else {
                    vec![]
                };

                let mut new_locks = Vec::<Lock>::new();
                let mut lock_to_delete = Lock {
                    id: "".to_owned(),
                    path: "".to_owned(),
                    owner: None,
                    locked_at: {
                        let locked_at: DateTime<Utc> = DateTime::<Utc>::MIN_UTC;
                        locked_at.to_rfc3339()
                    },
                };

                for lock in locks_from_data.iter() {
                    if lock.id == *id {
                        if Option::is_some(&lock.owner) && !force {
                            return Err(GitLFSError::GeneralError("".to_string()));
                        }
                        lock_to_delete.id = lock.id.to_owned();
                        lock_to_delete.path = lock.path.to_owned();
                        lock_to_delete.owner = lock.owner.clone();
                        lock_to_delete.locked_at = lock.locked_at.to_owned();
                    } else if !lock.id.is_empty() {
                        new_locks.push(Lock {
                            id: lock.id.to_owned(),
                            path: lock.path.to_owned(),
                            owner: lock.owner.clone(),
                            locked_at: lock.locked_at.to_owned(),
                        });
                    }
                }
                if lock_to_delete.id.is_empty() {
                    return Err(GitLFSError::GeneralError("".to_string()));
                }

                // No locks remains, delete the repo from database.
                if new_locks.is_empty() {
                    locks::Entity::delete_by_id(repo.to_owned())
                        .exec(self.get_connection())
                        .await
                        .unwrap();

                    return Ok(lock_to_delete);
                }

                // Update remaining locks.
                let data = serde_json::to_string(&new_locks).unwrap();

                let mut lock_to: locks::ActiveModel = val.into();
                lock_to.data = Set(data.to_owned());
                let res = lock_to.update(self.get_connection()).await;
                match res.is_ok() {
                    true => Ok(lock_to_delete),
                    false => Err(GitLFSError::GeneralError("".to_string())),
                }
            }
            // Not exist, error.
            None => Err(GitLFSError::GeneralError("".to_string())),
        }
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
        let docs = repo_directory::new(pid, "docs", "/docs");
        let third_parts = repo_directory::new(pid, "third_parts", "/third_parts");
        let projects = repo_directory::new(pid, "projects", "/projects");
        let model_vec = vec![docs, third_parts, projects];
        repo_directory::Entity::insert_many(model_vec)
            .on_conflict(
                OnConflict::column(repo_directory::Column::FullPath)
                    .do_nothing()
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
