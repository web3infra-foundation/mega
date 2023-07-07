//!
//!
//!
//!
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;

use entity::commit;
use entity::locks;
use entity::meta;
use entity::node;
use entity::refs;

use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::DatabaseBackend;
use sea_orm::DatabaseConnection;
use sea_orm::DbErr;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::Set;
use sea_orm::Statement;

use crate::driver::lfs::storage::MetaObject;
use crate::driver::lfs::structs::Lock;
use crate::driver::lfs::structs::RequestVars;
use crate::driver::MegaError;
use crate::driver::ObjectStorage;

use common::errors::GitLFSError;
use common::utils::ZERO_ID;

#[derive(Debug, Default, Clone)]
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
    async fn get_head_object_id(&self, repo_path: &Path) -> String {
        let path_str = repo_path.to_str().unwrap();
        let refs_list = self.search_refs(path_str).await.unwrap();

        if refs_list.is_empty() {
            ZERO_ID.to_string()
        } else {
            for refs in &refs_list {
                if repo_path.to_str().unwrap() == refs.repo_path {
                    return refs.ref_git_id.clone();
                }
            }
            for refs in &refs_list {
                // if repo_path is subdirectory of some commit, we should generae a fake commit
                if repo_path.starts_with(refs.repo_path.clone()) {
                    // return self.generate_child_commit_and_refs(refs, repo_path).await;
                }
            }
            //situation: repo_path: root/repotest2/src, commit: root/repotest
            ZERO_ID.to_string()
        }
    }

    async fn get_ref_object_id(&self, repo_path: &Path) -> HashMap<String, String> {
        // assuming HEAD points to branch master.
        let mut map = HashMap::new();
        let refs: Vec<refs::Model> = refs::Entity::find()
            .filter(refs::Column::RepoPath.eq(repo_path.to_str()))
            .all(&self.connection)
            .await
            .unwrap();
        for git_ref in refs {
            map.insert(git_ref.ref_git_id, git_ref.ref_name);
        }
        map
    }

    async fn get_full_pack_data(&self, _repo_path: &Path) -> Result<Vec<u8>, MegaError> {
        todo!()
    }

    async fn get_incremental_pack_data(
        &self,
        _repo_path: &Path,
        _want: &HashSet<String>,
        _have: &HashSet<String>,
    ) -> Result<Vec<u8>, MegaError> {
        todo!()
    }

    async fn get_commit_by_hash(&self, _hash: &str) -> Result<Vec<u8>, MegaError> {
        todo!()
    }

    async fn get_commit_by_id(&self, git_id: String) -> Result<commit::Model, MegaError> {
        Ok(commit::Entity::find()
            .filter(commit::Column::GitId.eq(git_id))
            .one(&self.connection)
            .await
            .unwrap()
            .unwrap())
    }

    async fn get_all_commits_by_path(&self, path: &Path) -> Result<Vec<commit::Model>, MegaError> {
        let commits: Vec<commit::Model> = commit::Entity::find()
            .filter(commit::Column::RepoPath.eq(path.to_str().unwrap()))
            .all(&self.connection)
            .await
            .unwrap();
        Ok(commits)
    }

    async fn get_hash_object(&self, _hash: &str) -> Result<Vec<u8>, MegaError> {
        todo!()
    }

    async fn save_refs(&self, save_models: Vec<refs::ActiveModel>) {
        batch_save_model(&self.connection, save_models)
            .await
            .unwrap();
    }

    async fn update_refs(&self, old_id: String, new_id: String, path: &Path) {
        let ref_data: Option<refs::Model> = refs::Entity::find()
            .filter(refs::Column::RefGitId.eq(old_id))
            .filter(refs::Column::RepoPath.eq(path.to_str().unwrap()))
            .one(&self.connection)
            .await
            .unwrap();
        let mut ref_data: refs::ActiveModel = ref_data.unwrap().into();
        ref_data.ref_git_id = Set(new_id);
        ref_data.updated_at = Set(chrono::Utc::now().naive_utc());
        ref_data.update(&self.connection).await.unwrap();
    }

    async fn delete_refs(&self, old_id: String, path: &Path) {
        let delete_ref = refs::ActiveModel {
            ref_git_id: Set(old_id),
            repo_path: Set(path.to_str().unwrap().to_owned()),
            ..Default::default()
        };
        refs::Entity::delete(delete_ref)
            .exec(&self.connection)
            .await
            .unwrap();
    }

    async fn save_nodes(&self, nodes: Vec<node::ActiveModel>) -> Result<bool, anyhow::Error> {
        let conn = &self.connection;
        let mut sum = 0;
        let mut batch_nodes = Vec::new();
        for node in nodes {
            // let model = node.try_into_model().unwrap();
            let size = node.data.as_ref().len();
            let limit = 10 * 1024 * 1024;
            if sum + size < limit && batch_nodes.len() < 50 {
                sum += size;
                batch_nodes.push(node);
            } else {
                node::Entity::insert_many(batch_nodes)
                    .exec(conn)
                    .await
                    .unwrap();
                sum = 0;
                batch_nodes = vec![node];
            }
        }
        if !batch_nodes.is_empty() {
            node::Entity::insert_many(batch_nodes)
                .exec(conn)
                .await
                .unwrap();
        }
        Ok(true)
    }

    async fn save_commits(&self, commits: Vec<commit::ActiveModel>) -> Result<bool, anyhow::Error> {
        let conn = &self.connection;
        batch_save_model(conn, commits).await.unwrap();
        Ok(true)
    }

    async fn lfs_get_meta(&self, v: &RequestVars) -> Result<MetaObject, GitLFSError> {
        let result = meta::Entity::find_by_id(v.oid.clone())
            .one(&self.connection)
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
            .one(&self.connection)
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

        let res = meta::Entity::insert(meta_to).exec(&self.connection).await;
        match res {
            Ok(_) => Ok(meta),
            Err(_) => Err(GitLFSError::GeneralError("".to_string())),
        }
    }

    async fn lfs_delete_meta(&self, v: &RequestVars) -> Result<(), GitLFSError> {
        let res = meta::Entity::delete_by_id(v.oid.to_owned())
            .exec(&self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(_) => Err(GitLFSError::GeneralError("".to_string())),
        }
    }

    async fn lfs_get_locks(&self, refspec: &str) -> Result<Vec<Lock>, GitLFSError> {
        let result = locks::Entity::find_by_id(refspec)
            .one(&self.connection)
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
            .one(&self.connection)
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
                let res = lock_to.update(&self.connection).await;
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
                let res = locks::Entity::insert(lock_to).exec(&self.connection).await;
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
            .one(&self.connection)
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
                        .exec(&self.connection)
                        .await
                        .unwrap();

                    return Ok(lock_to_delete);
                }

                // Update remaining locks.
                let data = serde_json::to_string(&new_locks).unwrap();

                let mut lock_to: locks::ActiveModel = val.into();
                lock_to.data = Set(data.to_owned());
                let res = lock_to.update(&self.connection).await;
                match res.is_ok() {
                    true => Ok(lock_to_delete),
                    false => Err(GitLFSError::GeneralError("".to_string())),
                }
            }
            // Not exist, error.
            None => Err(GitLFSError::GeneralError("".to_string())),
        }
    }
}

impl MysqlStorage {
    #[allow(unused)]
    async fn search_refs(&self, path_str: &str) -> Result<Vec<refs::Model>, DbErr> {
        refs::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DatabaseBackend::MySql,
            r#"SELECT * FROM gust.refs where ? LIKE CONCAT(repo_path, '%') and ref_name = 'refs/heads/master' "#,
            [path_str.into()],
        ))
        .all(&self.connection)
        .await
    }

    #[allow(unused)]
    async fn search_commits(&self, path_str: &str) -> Result<Vec<commit::Model>, DbErr> {
        commit::Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DatabaseBackend::MySql,
                r#"SELECT * FROM gust.commit where ? LIKE CONCAT(repo_path, '%')"#,
                [path_str.into()],
            ))
            .all(&self.connection)
            .await
    }

    /// Generates a new commit for a subdirectory of the original project directory.
    /// Steps:
    /// 1. Retrieve the root commit based on the provided reference's Git ID.
    /// 2. If a root tree is found by searching for the repository path:
    ///    a. Construct a child commit using the retrieved root commit and the root tree.
    ///    b. Save the child commit.
    ///    c. Obtain the commit ID of the child commit.
    ///    d. Construct a child reference with the repository path, reference name, commit ID, and other relevant information.
    ///    e. Save the child reference in the database.
    /// 3. Return the commit ID of the child commit if successful; otherwise, return a default ID.
    // async fn generate_child_commit_and_refs(&self, refs: &refs::Model, repo_path: &Path) -> String {
    //     if let Some(root_tree) = self.search_root_node_by_path(repo_path).await {
    //         let root_commit = self.get_commit_by_id(refs.ref_git_id).await.unwrap();
    //         let child_commit = Commit::build_from_model_and_root(&root_commit, root_tree);
    //         self.save_commits(&vec![child_commit.clone()], repo_path)
    //             .await
    //             .unwrap();
    //         let commit_id = child_commit.meta.id.to_plain_str();
    //         let child_refs = refs::ActiveModel {
    //             id: NotSet,
    //             repo_path: Set(repo_path.to_str().unwrap().to_string()),
    //             ref_name: Set(refs.ref_name.clone()),
    //             ref_git_id: Set(commit_id.clone()),
    //             created_at: Set(chrono::Utc::now().naive_utc()),
    //             updated_at: Set(chrono::Utc::now().naive_utc()),
    //         };
    //         batch_save_model(&self.connection, vec![child_refs])
    //             .await
    //             .unwrap();
    //         commit_id
    //     } else {
    //         ZERO_ID.to_string()
    //     }
    // }

    #[allow(unused)]
    async fn search_root_node_by_path(&self, repo_path: &Path) -> Option<node::Model> {
        tracing::debug!("file_name: {:?}", repo_path.file_name());
        let res = node::Entity::find()
            .filter(node::Column::Name.eq(repo_path.file_name().unwrap().to_str().unwrap()))
            .one(&self.connection)
            .await
            .unwrap();
        if let Some(res) = res {
            Some(res)
        } else {
            node::Entity::find()
                // .filter(node::Column::Path.eq(repo_path.to_str().unwrap()))
                .filter(node::Column::Name.eq(""))
                .one(&self.connection)
                .await
                .unwrap()
        }
    }

    #[allow(unused)]
    async fn get_node_by_id(&self, id: &str) -> Option<node::Model> {
        node::Entity::find()
            .filter(node::Column::GitId.eq(id))
            .one(&self.connection)
            .await
            .unwrap()
    }

    // async fn get_nodes_by_ids(&self, ids: Vec<String>) -> HashMap<Hash, node::Model> {
    //     node::Entity::find()
    //         .filter(node::Column::GitId.is_in(ids))
    //         .all(&self.connection)
    //         .await
    //         .unwrap()
    //         .into_iter()
    //         .map(|f| (Hash::from_str(&f.git_id).unwrap(), f))
    //         .collect()
    // }

    // retrieve all sub trees recursively
    // #[async_recursion]
    // async fn get_child_trees(&self, root: &node::Model, hash_meta: &mut HashMap<String, MetaData>) {
    //     let t = Tree::new(Arc::new(MetaData::new(ObjectType::Tree, &root.data)));
    //     let mut child_ids = vec![];
    //     for item in t.tree_items {
    //         if !hash_meta.contains_key(&item.id.to_plain_str()) {
    //             child_ids.push(item.id.to_plain_str());
    //         }
    //     }
    //     let childs = node::Entity::find()
    //         .filter(node::Column::GitId.is_in(child_ids))
    //         .all(&self.connection)
    //         .await
    //         .unwrap();
    //     for c in childs {
    //         if c.node_type == "tree" {
    //             self.get_child_trees(&c, hash_meta).await;
    //         } else {
    //             let b_meta = MetaData::new(ObjectType::Blob, &c.data);
    //             hash_meta.insert(b_meta.id.to_plain_str(), b_meta);
    //         }
    //     }
    //     let t_meta = t.meta;
    //     tracing::info!("{}, {}", t_meta.id, t.tree_name);
    //     hash_meta.insert(t_meta.id.to_plain_str(), Arc::try_unwrap(t_meta).unwrap());
    // }
}

// mysql sea_orm bathc insert
async fn batch_save_model<E, A>(
    conn: &DatabaseConnection,
    save_models: Vec<A>,
) -> Result<(), anyhow::Error>
where
    E: EntityTrait,
    A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
{
    let mut futures = Vec::new();

    // notice that sqlx not support packets larger than 16MB now
    for chunk in save_models.chunks(100) {
        let save_result = E::insert_many(chunk.iter().cloned()).exec(conn).await;
        futures.push(save_result);
    }
    // futures::future::join_all(futures).await;
    Ok(())
}
