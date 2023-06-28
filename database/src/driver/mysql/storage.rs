use crate::driver::lfs::storage::MetaObject;
use crate::driver::lfs::structs::Lock;
use crate::driver::lfs::structs::RequestVars;
use crate::driver::MegaError;
use crate::driver::ObjectStorage;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use common::errors::GitLFSError;
use entity::locks;
use entity::meta;
use sea_orm::ActiveModelTrait;
use sea_orm::DatabaseConnection;
use sea_orm::EntityTrait;
use sea_orm::Set;
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::path::Path;

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
    async fn get_head_object_id(&self, _path: &Path) -> String {
        todo!()
    }

    async fn get_ref_object_id(&self, _path: &Path) -> HashMap<String, String> {
        todo!()
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

    async fn get_hash_object(&self, _hash: &str) -> Result<Vec<u8>, MegaError> {
        todo!()
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
